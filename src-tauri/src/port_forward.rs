use crate::AppState;
use russh::client::Handle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::net::TcpListener;
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

type SshHandle = Handle<crate::ClientHandler<tauri::Wry>>;

pub struct PortForwardRuntime {
    pub id: String,
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    cancel: watch::Sender<bool>,
    task: JoinHandle<()>,
}

pub type PortForwardMap = Arc<Mutex<HashMap<String, HashMap<String, PortForwardRuntime>>>>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PortForwardInfo {
    pub id: String,
    pub local_host: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

impl PortForwardInfo {
    fn from_runtime(rt: &PortForwardRuntime) -> Self {
        Self {
            id: rt.id.clone(),
            local_host: rt.local_host.clone(),
            local_port: rt.local_port,
            remote_host: rt.remote_host.clone(),
            remote_port: rt.remote_port,
        }
    }
}

pub fn new_port_forward_map() -> PortForwardMap {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn stop_all_for_session(map: &PortForwardMap, session_id: &str) {
    let mut guard = map.lock().await;
    if let Some(forwards) = guard.remove(session_id) {
        for (_, rt) in forwards {
            let _ = rt.cancel.send(true);
            rt.task.abort();
        }
    }
}

async fn get_session_handle(state: &AppState, session_id: &str) -> Result<Arc<Mutex<SshHandle>>, String> {
    let sessions = state.sessions.lock().await;
    let sess = sessions.get(session_id).ok_or("Session not found")?;
    Ok(sess.handle.clone())
}

async fn run_local_forward(
    handle: Arc<Mutex<SshHandle>>,
    local_host: String,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
    mut cancel: watch::Receiver<bool>,
) {
    let bind_addr = format!("{}:{}", local_host, local_port);
    let listener = match TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[port-forward] bind {} failed: {}", bind_addr, e);
            return;
        }
    };

    loop {
        tokio::select! {
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    break;
                }
            }
            accept = listener.accept() => {
                let Ok((mut tcp, addr)) = accept else { continue };
                let handle = handle.clone();
                let remote_host = remote_host.clone();
                tokio::spawn(async move {
                    let channel = {
                        let handle_guard = handle.lock().await;
                        handle_guard
                            .channel_open_direct_tcpip(
                                &remote_host,
                                remote_port as u32,
                                &addr.ip().to_string(),
                                addr.port() as u32,
                            )
                            .await
                    };
                    let Ok(channel) = channel else { return };
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                });
            }
        }
    }
}

#[tauri::command]
pub async fn start_port_forward(
    state: State<'_, AppState>,
    session_id: String,
    local_host: String,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
) -> Result<PortForwardInfo, String> {
    if state.local_sessions.lock().await.contains_key(&session_id) {
        return Err("本地终端不支持端口转发".into());
    }

    {
        let sessions = state.sessions.lock().await;
        if !sessions.contains_key(&session_id) {
            return Err("Session not found".into());
        }
    }

    if remote_host.trim().is_empty() {
        return Err("远程主机不能为空".into());
    }
    if remote_port == 0 {
        return Err("远程端口无效".into());
    }

    let bind_host = if local_host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        local_host.trim().to_string()
    };

    let handle = get_session_handle(state.inner(), &session_id).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let (cancel_tx, cancel_rx) = watch::channel(false);

    let task = tokio::spawn(run_local_forward(
        handle,
        bind_host.clone(),
        local_port,
        remote_host.clone(),
        remote_port,
        cancel_rx,
    ));

    let runtime = PortForwardRuntime {
        id: id.clone(),
        local_host: bind_host,
        local_port,
        remote_host,
        remote_port,
        cancel: cancel_tx,
        task,
    };

    let info = PortForwardInfo::from_runtime(&runtime);
    state
        .port_forwards
        .lock()
        .await
        .entry(session_id)
        .or_default()
        .insert(id, runtime);

    Ok(info)
}

#[tauri::command]
pub async fn stop_port_forward(
    state: State<'_, AppState>,
    session_id: String,
    forward_id: String,
) -> Result<(), String> {
    let mut guard = state.port_forwards.lock().await;
    let Some(forwards) = guard.get_mut(&session_id) else {
        return Ok(());
    };
    if let Some(rt) = forwards.remove(&forward_id) {
        let _ = rt.cancel.send(true);
        rt.task.abort();
    }
    if forwards.is_empty() {
        guard.remove(&session_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn list_port_forwards(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<PortForwardInfo>, String> {
    let guard = state.port_forwards.lock().await;
    let Some(forwards) = guard.get(&session_id) else {
        return Ok(Vec::new());
    };
    Ok(forwards.values().map(PortForwardInfo::from_runtime).collect())
}
