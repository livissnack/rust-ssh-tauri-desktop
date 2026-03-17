use libp2p::{
    futures::StreamExt,
    mdns, noise, request_response, tcp, yamux, PeerId, swarm::NetworkBehaviour, swarm::SwarmEvent,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tauri::Emitter;
use tokio::sync::mpsc;
use std::path::PathBuf;
use serde_json::json;
use std::sync::Arc;
use redb::{Database, ReadableTable};
use uuid::Uuid;
use std::sync::Mutex;
use std::collections::HashSet;
use crate::AppState;
use crate::{P2P_REMARKS_TABLE, P2P_MESSAGES_TABLE};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessageRecord {
    pub id: String,
    pub self_id: String,
    pub peer_id: String,
    pub content: String,
    pub msg_type: String,
    pub direction: String,
    pub timestamp: u64,
}

pub struct P2PStatus {
    pub online_peers: Mutex<HashSet<String>>,
}

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

#[tauri::command]
pub async fn get_online_peers(
    state: tauri::State<'_, Arc<P2PStatus>>
) -> Result<Vec<String>, String> {
    let peers = state.online_peers.lock().map_err(|e| e.to_string())?;
    let list: Vec<String> = peers.iter().cloned().collect();
    Ok(list)
}

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
) -> Result<Vec<ChatMessageRecord>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(P2P_MESSAGES_TABLE).map_err(|e| e.to_string())?;

    let kw = keyword.to_lowercase();
    let mut results = Vec::new();

    let iter = table.iter().map_err(|e| e.to_string())?;

    for result in iter {
        let (_, value) = result.map_err(|e| e.to_string())?;
        let val_str = value.value();

        if val_str.contains(&peer_id) && val_str.to_lowercase().contains(&kw) {
            if let Ok(msg) = serde_json::from_str::<ChatMessageRecord>(val_str) {
                if msg.peer_id == peer_id {
                    results.push(msg);
                }
            }
        }
    }

    results.sort_by_key(|m| m.timestamp);

    Ok(results.into_iter().take(100).collect())
}

pub async fn start_p2p_node(
    app_handle: tauri::AppHandle,
    mut cmd_receiver: mpsc::UnboundedReceiver<P2PCommand>,
    db: Arc<Database>,
    status: Arc<P2PStatus>,
) -> Result<(), Box<dyn Error>> {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let self_id_str = local_peer_id.to_string();
    println!("[P2P] 正在启动节点，我的 PeerId: {}", self_id_str);

    let get_now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    };

    let _ = app_handle.emit("p2p-my-id", &self_id_str);

    // 0.56.0 推荐的 SwarmBuilder 写法
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id()
            )?;
            let chat = request_response::json::Behaviour::new(
                [(libp2p::StreamProtocol::new("/hiphup-chat/1.0"), request_response::ProtocolSupport::Full)],
                request_response::Config::default(),
            );
            Ok(HiphupBehaviour { mdns, chat })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60))) // 设置闲置超时为60秒
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
                                timestamp: get_now(),
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
                                    timestamp: get_now(),
                                };
                                let _ = save_msg(&db, &record);
                                swarm.behaviour_mut().chat.send_request(&peer_id, ChatRequest::File { name, data });
                            }
                        }
                    }
                }
            }

            event = swarm.select_next_some() => match event {
                // 处理 mDNS 发现
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, addr) in list {
                        let id_str = peer_id.to_string();
                        swarm.add_peer_address(peer_id, addr);

                        if let Ok(mut online_list) = status.online_peers.lock() {
                            if online_list.insert(id_str.clone()) {
                                println!("[P2P] 邻居上线: {}", id_str);
                                let _ = app_handle.emit("p2p-peer-discovered", id_str);
                            }
                        }
                    }
                }

                // 处理 mDNS 过期（真正的下线）
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _addr) in list {
                        let id_str = peer_id.to_string();
                        if let Ok(mut online_list) = status.online_peers.lock() {
                            if online_list.remove(&id_str) {
                                println!("[P2P] 邻居过期 (mDNS Expired): {}", id_str);
                                let _ = app_handle.emit("p2p-peer-expired", id_str);
                            }
                        }
                    }
                }

                // 物理连接关闭（不再标记为下线）
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    println!("[P2P] 物理连接已关闭: {}。只要 mDNS 没过期，依然在线。", peer_id);
                }

                // 处理消息收发
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
                            content,
                            msg_type: "text".into(),
                            direction: "receive".into(),
                            timestamp: get_now(),
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
                                timestamp: get_now(),
                            }
                        }
                    };

                    let _ = save_msg(&db, &record);
                    let _ = app_handle.emit("p2p-receive-msg", json!(record));
                    let _ = swarm.behaviour_mut().chat.send_response(channel, ChatResponse { status: "ok".into() });
                }

                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("[P2P] 本地监听地址: {}", address);
                }
                _ => {}
            }
        }
    }
}