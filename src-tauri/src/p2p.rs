use libp2p::{
    futures::StreamExt,
    mdns, noise, request_response, tcp, yamux, PeerId, swarm::NetworkBehaviour, swarm::SwarmEvent,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
// 💡 修正：WebviewWindow 才是 Tauri 2.0 正确的窗口类型
use tauri::{Emitter, WebviewWindow};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatRequest { pub content: String }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatResponse { pub status: String }

pub enum P2PCommand {
    SendMessage { target: String, content: String },
}

// 💡 修正：libp2p 0.52+ 会自动生成 HiphupBehaviourEvent
#[derive(NetworkBehaviour)]
pub struct HiphupBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub chat: request_response::json::Behaviour<ChatRequest, ChatResponse>,
}

pub async fn start_p2p_node(
    window: WebviewWindow, // 💡 修改为 WebviewWindow
    mut cmd_receiver: mpsc::UnboundedReceiver<P2PCommand>
) -> Result<(), Box<dyn Error>> {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let _ = window.emit("p2p-my-id", local_peer_id.to_string());

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            // 💡 修正：ProtocolName::new 移除了，现在直接用字符串或特定的 StreamProtocol
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
                            swarm.behaviour_mut().chat.send_request(&peer_id, ChatRequest { content });
                        }
                    }
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _) in list {
                        let _ = window.emit("p2p-peer-discovered", peer_id.to_string());
                    }
                }
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _) in list {
                        let _ = window.emit("p2p-peer-expired", peer_id.to_string());
                    }
                }
                // 💡 修正：Request 变体中的字段名现在是 channel 而不是 response_channel
                SwarmEvent::Behaviour(HiphupBehaviourEvent::Chat(request_response::Event::Message {
                    peer,
                    message: request_response::Message::Request { request, channel, .. },
                    ..
                })) => {
                    let _ = window.emit("p2p-receive-msg", serde_json::json!({
                        "from": peer.to_string(),
                        "content": request.content
                    }));
                    let _ = swarm.behaviour_mut().chat.send_response(channel, ChatResponse { status: "ok".into() });
                }
                _ => {}
            }
        }
    }
}