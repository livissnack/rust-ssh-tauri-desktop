use libp2p::{
    futures::StreamExt,
    mdns, noise, request_response, tcp, yamux, PeerId, swarm::NetworkBehaviour, swarm::SwarmEvent,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tauri::{Emitter, WebviewWindow};
use tokio::sync::mpsc;
use std::path::PathBuf;
use serde_json::json;
use std::sync::Arc;
use redb::{Database, ReadableTable}; // 💡 导入 ReadableTable 以便使用 .iter()
use uuid::Uuid;
use crate::AppState;
use crate::{P2P_REMARKS_TABLE, P2P_MESSAGES_TABLE};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessageRecord {
    pub id: String,
    pub self_id: String,
    pub peer_id: String,
    pub content: String,
    pub msg_type: String,     // "text" | "file"
    pub direction: String,    // "send" | "receive"
    pub timestamp: u64,
}

// 内部工具函数：保存消息到数据库
fn save_msg(db: &Arc<Database>, record: &ChatMessageRecord) -> Result<(), Box<dyn Error>> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(P2P_MESSAGES_TABLE)?;
        let json_str = serde_json::to_string(record)?;
        table.insert(record.id.as_str(), json_str.as_str())?;
    }
    write_txn.commit()?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRequest {
    Text { content: String },
    File { name: String, data: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatResponse { pub status: String }

pub enum P2PCommand {
    SendMessage { target: String, content: String },
    SendFile { target: String, path: PathBuf },
}

#[derive(NetworkBehaviour)]
pub struct HiphupBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub chat: request_response::json::Behaviour<ChatRequest, ChatResponse>,
}

// --- Tauri Commands ---

#[tauri::command]
pub async fn set_p2p_remark(state: tauri::State<'_, AppState>, peer_id: String, remark: String) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(P2P_REMARKS_TABLE).map_err(|e| e.to_string())?;
        table.insert(peer_id.as_str(), remark.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_p2p_remarks(state: tauri::State<'_, AppState>) -> Result<std::collections::HashMap<String, String>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(P2P_REMARKS_TABLE).map_err(|e| e.to_string())?;
    let mut map = std::collections::HashMap::new();

    // 💡 显式处理迭代器错误，解决 E0282
    let iter = table.iter().map_err(|e| e.to_string())?;
    for result in iter {
        let (k, v) = result.map_err(|e| e.to_string())?;
        map.insert(k.value().to_string(), v.value().to_string());
    }
    Ok(map)
}

#[tauri::command]
pub async fn search_p2p_messages(
    state: tauri::State<'_, AppState>,
    peer_id: String,
    keyword: String,
) -> Result<Vec<ChatMessageRecord>, String> { // 💡 去掉 p2p:: 前缀
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(P2P_MESSAGES_TABLE).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let kw = keyword.to_lowercase();

    let iter = table.iter().map_err(|e| e.to_string())?;
    for result in iter {
        let (_, value) = result.map_err(|e| e.to_string())?;
        // 💡 这里的 ChatMessageRecord 直接引用当前作用域的结构体
        let msg: ChatMessageRecord = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if msg.peer_id == peer_id && msg.content.to_lowercase().contains(&kw) {
            results.push(msg);
        }
    }
    results.sort_by_key(|m| m.timestamp);
    Ok(results)
}

// --- Node Logic ---

pub async fn start_p2p_node(
    window: WebviewWindow,
    mut cmd_receiver: mpsc::UnboundedReceiver<P2PCommand>,
    db: Arc<Database>,
) -> Result<(), Box<dyn Error>> {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let self_id_str = local_peer_id.to_string();

    let _ = window.emit("p2p-my-id", &self_id_str);

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            let chat = request_response::json::Behaviour::new(
                [(libp2p::StreamProtocol::new("/hiphup-chat/1.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );
            Ok(HiphupBehaviour { mdns, chat })
        })?
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        tokio::select! {
            Some(cmd) = cmd_receiver.recv() => {
                match cmd {
                    P2PCommand::SendMessage { target, content } => {
                        if let Ok(peer_id) = target.parse::<PeerId>() {
                            let record = ChatMessageRecord {
                                id: Uuid::new_v4().to_string(),
                                self_id: self_id_str.clone(),
                                peer_id: target.clone(),
                                content: content.clone(),
                                msg_type: "text".into(),
                                direction: "send".into(),
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
                            };
                            let _ = save_msg(&db, &record);
                            swarm.behaviour_mut().chat.send_request(&peer_id, ChatRequest::Text { content });
                        }
                    }
                    P2PCommand::SendFile { target, path } => {
                         if let Ok(peer_id) = target.parse::<PeerId>() {
                            if let Ok(data) = std::fs::read(&path) {
                                let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                                let record = ChatMessageRecord {
                                    id: Uuid::new_v4().to_string(),
                                    self_id: self_id_str.clone(),
                                    peer_id: target.clone(),
                                    content: name.clone(),
                                    msg_type: "file".into(),
                                    direction: "send".into(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
                                };
                                let _ = save_msg(&db, &record);
                                swarm.behaviour_mut().chat.send_request(&peer_id, ChatRequest::File { name, data });
                            }
                        }
                    }
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, addr) in list {
                        swarm.add_peer_address(peer_id, addr);
                        let _ = window.emit("p2p-peer-discovered", peer_id.to_string());
                    }
                }
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Chat(request_response::Event::Message {
                    peer,
                    message: request_response::Message::Request { request, channel, .. },
                    ..
                })) => {
                    let peer_id_str = peer.to_string();
                    let record = match request {
                        ChatRequest::Text { content } => ChatMessageRecord {
                            id: Uuid::new_v4().to_string(),
                            self_id: self_id_str.clone(),
                            peer_id: peer_id_str,
                            content: content,
                            msg_type: "text".into(),
                            direction: "receive".into(),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        },
                        ChatRequest::File { name, data } => {
                            let _ = std::fs::write(format!("./{}", name), data);
                            ChatMessageRecord {
                                id: Uuid::new_v4().to_string(),
                                self_id: self_id_str.clone(),
                                peer_id: peer_id_str,
                                content: name,
                                msg_type: "file".into(),
                                direction: "receive".into(),
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            }
                        }
                    };

                    let _ = save_msg(&db, &record);
                    let _ = window.emit("p2p-receive-msg", json!(record));
                    let _ = swarm.behaviour_mut().chat.send_response(channel, ChatResponse { status: "ok".into() });
                }
                _ => {}
            }
        }
    }
}