use crate::app_state::{ActiveSession, AppState};
use crate::known_hosts::{HostKeyRole, HostKeyVerifier, HOST_KEY_PROMPT_TIMEOUT_SECS};
use crate::local_shell;
use crate::port_forward::stop_all_for_session;
use crate::servers::{get_servers, ServerConfig};
use russh::*;
use russh::client::DisconnectReason;
use serde::Serialize;
use std::future::Future;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Runtime, State, Window};
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Serialize, Clone)]
struct SshPayload {
    server_id: String,
    session_id: String,
    data: String,
}

pub struct ClientHandler<R: Runtime> {
    window: tauri::Window<R>,
    server_id: String,
    session_id: String,
    shell_channel_id: Arc<Mutex<Option<ChannelId>>>,
    host_key_verifier: Arc<HostKeyVerifier>,
}

impl<R: Runtime> client::Handler for ClientHandler<R> {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let verifier = self.host_key_verifier.clone();
        let key = server_public_key.clone();
        async move {
            match verifier.verify(&key).await {
                Ok(trusted) => Ok(trusted),
                Err(e) => Err(russh::Error::InvalidConfig(e)),
            }
        }
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let shell_id_opt = *self.shell_channel_id.lock().await;

        if Some(channel) == shell_id_opt {
            let text = String::from_utf8_lossy(data).to_string();
            let _ = self.window.emit(
                "ssh-output",
                SshPayload {
                    server_id: self.server_id.clone(),
                    session_id: self.session_id.clone(),
                    data: text,
                },
            );
        }
        Ok(())
    }

    async fn disconnected(&mut self, _reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        let _ = self.window.emit(
            "ssh-closed",
            serde_json::json!({
                "server_id": self.server_id,
                "session_id": self.session_id,
            }),
        );
        Ok(())
    }
}

const SSH_KEEPALIVE_INTERVAL_SECS: u64 = 30;
const SSH_KEEPALIVE_MAX: usize = 3;

fn build_ssh_client_config(channel_buffer_size: usize) -> client::Config {
    let mut config = client::Config::default();
    config.channel_buffer_size = channel_buffer_size;
    config.nodelay = true;
    config.keepalive_interval = Some(Duration::from_secs(SSH_KEEPALIVE_INTERVAL_SECS));
    config.keepalive_max = SSH_KEEPALIVE_MAX;
    config
}

async fn authenticate<H: client::Handler>(
    handle: &mut client::Handle<H>,
    config: &ServerConfig,
) -> Result<(), String> {
    if config.auth_type == "key" {
        let path_str = config.private_key_path.as_ref().ok_or("未配置私钥路径")?;
        let key_pair = russh::keys::load_secret_key(path_str, None)
            .map_err(|e| format!("加载私钥失败: {}", e))?;

        let hash_alg = if key_pair.algorithm().is_rsa() {
            handle
                .best_supported_rsa_hash()
                .await
                .map_err(|e| format!("获取 RSA 签名算法失败: {}", e))?
                .flatten()
        } else {
            None
        };

        let key_with_alg = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg);

        let auth_res = handle
            .authenticate_publickey(&config.username, key_with_alg)
            .await
            .map_err(|e| format!("私钥认证出错: {}", e))?;

        if !matches!(auth_res, russh::client::AuthResult::Success) {
            return Err("私钥认证被拒绝".into());
        }
    } else {
        let pass = config.password.as_deref().unwrap_or("");
        let auth_res = handle
            .authenticate_password(&config.username, pass)
            .await
            .map_err(|e| format!("密码认证出错: {}", e))?;

        if !matches!(auth_res, russh::client::AuthResult::Success) {
            return Err("用户名或密码错误".into());
        }
    }
    Ok(())
}

struct LatencyProbeHandler {
    host_key_verifier: Arc<HostKeyVerifier>,
}

impl client::Handler for LatencyProbeHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let verifier = self.host_key_verifier.clone();
        let key = server_public_key.clone();
        async move {
            match verifier.verify(&key).await {
                Ok(trusted) => Ok(trusted),
                Err(e) => Err(russh::Error::InvalidConfig(e)),
            }
        }
    }
}

fn make_host_key_verifier(
    state: &AppState,
    config: &ServerConfig,
    allow_prompt: bool,
    window_label: Option<String>,
    host_role: HostKeyRole,
) -> Arc<HostKeyVerifier> {
    Arc::new(HostKeyVerifier::new(
        state.db.clone(),
        state.app_handle.clone(),
        state.host_key_hub.clone(),
        config.host.clone(),
        config.port,
        config.name.clone(),
        allow_prompt,
        window_label,
        host_role,
    ))
}

fn make_client_handler<R: Runtime>(
    window: &tauri::Window<R>,
    target_config: &ServerConfig,
    state: &AppState,
    session_id: &str,
    shell_channel_id: &Arc<Mutex<Option<ChannelId>>>,
    host_role: HostKeyRole,
) -> ClientHandler<R> {
    ClientHandler {
        window: window.clone(),
        server_id: target_config.id.clone(),
        session_id: session_id.to_string(),
        shell_channel_id: shell_channel_id.clone(),
        host_key_verifier: make_host_key_verifier(
            state,
            target_config,
            true,
            Some(window.label().to_string()),
            host_role,
        ),
    }
}

async fn open_probe_session(
    state: &AppState,
    target_config: &ServerConfig,
    all_configs: &[ServerConfig],
) -> Result<client::Handle<LatencyProbeHandler>, String> {
    let config = build_ssh_client_config(1024);
    let client_config = Arc::new(config);
    let connect_timeout = Duration::from_secs(8);
    let handler = LatencyProbeHandler {
        host_key_verifier: make_host_key_verifier(
            state,
            target_config,
            false,
            None,
            HostKeyRole::Direct,
        ),
    };

    match target_config.jump_host_id.as_deref() {
        None | Some("") => {
            let addr = format!("{}:{}", target_config.host, target_config.port);
            let mut handle = timeout(connect_timeout, client::connect(client_config, addr, handler))
                .await
                .map_err(|_| format!("连接 {} 超时", target_config.host))?
                .map_err(|e| format!("直连失败: {}", e))?;
            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
        Some(jump_id) => {
            let jump_config = all_configs
                .iter()
                .find(|s| s.id == jump_id)
                .ok_or(format!("找不到跳板机: {}", jump_id))?;

            let jump_handle = Box::pin(open_probe_session(state, jump_config, all_configs)).await?;

            let channel = timeout(
                Duration::from_secs(8),
                jump_handle.channel_open_direct_tcpip(
                    &target_config.host,
                    target_config.port as u32,
                    "127.0.0.1",
                    0,
                ),
            )
            .await
            .map_err(|_| "跳板机建立隧道响应超时".to_string())?
            .map_err(|e| format!("隧道建立失败: {}", e))?;

            let mut handle = timeout(
                connect_timeout,
                client::connect_stream(
                    client_config,
                    channel.into_stream(),
                    LatencyProbeHandler {
                        host_key_verifier: make_host_key_verifier(
                            state,
                            target_config,
                            false,
                            None,
                            HostKeyRole::Target,
                        ),
                    },
                ),
            )
            .await
            .map_err(|_| format!("隧道内与目标 {} 握手超时", target_config.host))?
            .map_err(|e| format!("隧道内握手失败: {}", e))?;

            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
    }
}

pub async fn measure_server_latency(
    state: &AppState,
    target_config: &ServerConfig,
    all_configs: &[ServerConfig],
) -> Result<u32, String> {
    let start = Instant::now();
    let _handle = open_probe_session(state, target_config, all_configs).await?;
    Ok(start.elapsed().as_millis() as u32)
}

async fn create_recursive_session<R: Runtime>(
    window: tauri::Window<R>,
    state: &AppState,
    target_config: &ServerConfig,
    all_configs: &Vec<ServerConfig>,
    session_id: String,
    shell_channel_id: Arc<Mutex<Option<ChannelId>>>,
    is_final_target: bool,
) -> Result<client::Handle<ClientHandler<R>>, String> {
    let config = build_ssh_client_config(4096);
    let client_config = Arc::new(config);
    let connect_timeout = Duration::from_secs(HOST_KEY_PROMPT_TIMEOUT_SECS + 15);

    let direct_host_role = if is_final_target {
        HostKeyRole::Direct
    } else {
        HostKeyRole::Jump
    };
    let tunnel_host_role = if is_final_target {
        HostKeyRole::Target
    } else {
        HostKeyRole::Jump
    };

    match target_config.jump_host_id.as_deref() {
        None | Some("") => {
            let addr = format!("{}:{}", target_config.host, target_config.port);
            let handler = make_client_handler(
                &window,
                target_config,
                state,
                &session_id,
                &shell_channel_id,
                direct_host_role,
            );

            let mut handle = timeout(connect_timeout, client::connect(client_config, addr, handler))
                .await
                .map_err(|_| format!("连接目标 {} 超时", target_config.host))?
                .map_err(|e| format!("直连失败: {}", e))?;

            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
        Some(jump_id) => {
            let jump_config = all_configs
                .iter()
                .find(|s| s.id == jump_id)
                .ok_or(format!("找不到跳板机: {}", jump_id))?;

            let jump_handle = Box::pin(create_recursive_session(
                window.clone(),
                state,
                jump_config,
                all_configs,
                format!("{}_tunnel", session_id),
                shell_channel_id.clone(),
                false,
            ))
            .await?;
            println!(
                "隧道已建立，正在尝试在隧道内连接目标: {}:{}",
                target_config.host, target_config.port
            );
            let channel = timeout(
                Duration::from_secs(8),
                jump_handle.channel_open_direct_tcpip(
                    &target_config.host,
                    target_config.port as u32,
                    "127.0.0.1",
                    0,
                ),
            )
            .await
            .map_err(|_| "跳板机建立隧道响应超时".to_string())?
            .map_err(|e| format!("隧道建立失败: {}", e))?;

            let handler = make_client_handler(
                &window,
                target_config,
                state,
                &session_id,
                &shell_channel_id,
                tunnel_host_role,
            );

            let mut handle = timeout(
                connect_timeout,
                client::connect_stream(client_config, channel.into_stream(), handler),
            )
            .await
            .map_err(|_| format!("隧道内与目标 {} 握手超时", target_config.host))?
            .map_err(|e| format!("隧道内握手失败: {}", e))?;

            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
    }
}

#[tauri::command]
pub async fn connect_ssh(
    window: Window,
    state: State<'_, AppState>,
    server_id: String,
    session_id: String,
) -> Result<(), String> {
    {
        let sessions = state.sessions.lock().await;
        if sessions.contains_key(&session_id) {
            return Ok(());
        }
    }
    let shell_id_container = Arc::new(Mutex::new(None));
    let servers = get_servers(state.clone()).await?;
    let target_config = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or("配置不存在")?
        .clone();
    let handle = create_recursive_session(
        window.clone(),
        state.inner(),
        &target_config,
        &servers,
        session_id.clone(),
        shell_id_container.clone(),
        true,
    )
    .await?;
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    {
        let mut id_lock = shell_id_container.lock().await;
        *id_lock = Some(channel.id());
    }
    channel
        .request_pty(true, "xterm", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_shell(true)
        .await
        .map_err(|e| e.to_string())?;
    let channel_id = channel.id();
    state.sessions.lock().await.insert(
        session_id,
        ActiveSession {
            handle: Arc::new(Mutex::new(handle)),
            channel_id,
            channel: Arc::new(Mutex::new(channel)),
            sftp: Arc::new(Mutex::new(None)),
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn write_to_ssh(
    state: State<'_, AppState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    if state.local_sessions.lock().await.contains_key(&session_id) {
        return local_shell::write_local_shell(&state.local_sessions, &session_id, data).await;
    }

    let channel_arc = {
        let sessions = state.sessions.lock().await;
        let sess = sessions.get(&session_id).ok_or("Session not found")?;
        sess.channel.clone()
    };

    let lock_timeout = Duration::from_secs(2);
    let write_timeout = Duration::from_secs(15);

    let channel = match timeout(lock_timeout, channel_arc.lock()).await {
        Ok(guard) => guard,
        Err(_) => return Err(format!("写入 SSH 通道锁超时: session_id={}", session_id)),
    };

    match timeout(write_timeout, channel.data(Cursor::new(data.into_bytes()))).await {
        Ok(res) => res.map_err(|e| format!("写入 SSH 通道失败: {:?}", e)),
        Err(_) => Err(format!("写入 SSH 写入超时: session_id={}", session_id)),
    }
}

#[tauri::command]
pub async fn disconnect_ssh(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    if state.local_sessions.lock().await.contains_key(&session_id) {
        return local_shell::disconnect_local_shell(&state.local_sessions, &session_id).await;
    }

    let session_opt = {
        let mut sessions = state.sessions.lock().await;
        sessions.remove(&session_id)
    };

    if let Some(session) = session_opt {
        let handle = session.handle.lock().await;
        let _ = handle
            .disconnect(
                russh::Disconnect::ByApplication,
                "User closed connection",
                "en",
            )
            .await;
    }
    stop_all_for_session(&state.port_forwards, &session_id).await;
    Ok(())
}

#[tauri::command]
pub async fn resize_ssh(
    state: State<'_, AppState>,
    session_id: String,
    rows: u32,
    cols: u32,
) -> Result<(), String> {
    if state.local_sessions.lock().await.contains_key(&session_id) {
        return local_shell::resize_local_shell(&state.local_sessions, &session_id, rows, cols).await;
    }

    let channel_arc = {
        let sessions = state.sessions.lock().await;
        sessions.get(&session_id).map(|sess| sess.channel.clone())
    };

    if let Some(channel_mutex) = channel_arc {
        let channel = channel_mutex.lock().await;
        channel
            .window_change(cols, rows, 0, 0)
            .await
            .map_err(|e| format!("同步 PTY 尺寸失败: {:?}", e))?;

        Ok(())
    } else {
        Ok(())
    }
}
