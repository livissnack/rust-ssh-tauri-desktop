mod sync;
mod p2p;
mod security;
mod redis_manager;
mod local_shell;
mod api_debugger;
use crate::sync::{
    get_sync_settings,
    save_sync_settings,
    sync_to_cloud,
    sync_from_cloud,
    schedule_push_sync,
    run_startup_pull,
};
use security::{encrypt_secret, decrypt_secret};
use p2p::{set_p2p_remark, start_p2p_node, get_p2p_remarks, search_p2p_messages, get_online_peers};
use local_shell::{LocalSessionMap, LOCAL_SERVER_ID};
use api_debugger::{
    export_api_debugger_file, get_api_debugger_data, import_api_debugger_file, save_api_debugger_data,
};
use redis_manager::{redis_connect, redis_get_keys, redis_get_value, redis_set_value, redis_del_key, redis_rename_key, redis_get_ttl, redis_get_type, save_redis_config, get_redis_configs, delete_redis_config, clear_all_redis_configs};
use tokio::sync::mpsc;
use russh::*;
use russh::client::DisconnectReason;
use std::sync::Arc;
use tauri::{Emitter, Window, Runtime, State, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Mutex;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use redb::{Database, TableDefinition, ReadableTable};
use uuid::Uuid;
use std::future::Future;
use tokio::time::{timeout};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use futures::StreamExt;
use serde_json::json;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub const SERVERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ssh_servers");
pub const COMMANDS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("quick_commands");
pub const AI_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ai_settings");
pub const SYNC_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sync_config");
pub const REDIS_CONN_TABLE: TableDefinition<&str, &str> = TableDefinition::new("redis_connections");
pub const P2P_MESSAGES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("p2p_messages");
pub const P2P_REMARKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("p2p_remarks");
pub const API_DEBUGGER_TABLE: TableDefinition<&str, &str> = TableDefinition::new("api_debugger");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    pub auth_type: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub jump_host_id: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    pub updated_at: u64,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuickCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub group: Option<String>,
    pub updated_at: u64,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub current_provider: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Serialize, Clone)]
struct SshPayload {
    server_id: String,
    session_id: String,
    data: String,
}

#[derive(Serialize)]
struct FileInfo {
    name: String,
    is_dir: bool,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileDetail {
    path: String,
    name: String,
    is_dir: bool,
    size: u64,
    permissions: Option<String>,
    permissions_text: Option<String>,
    modified_at: Option<u64>,
    accessed_at: Option<u64>,
    uid: Option<u32>,
    gid: Option<u32>,
    user: Option<String>,
    group: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    #[serde(rename = "taskId")]
    task_id: String,
    progress: u64,
}

pub struct ClientHandler<R: Runtime> {
    window: tauri::Window<R>,
    server_id: String,
    session_id: String,
    shell_channel_id: Arc<Mutex<Option<ChannelId>>>,
}

impl<R: Runtime> client::Handler for ClientHandler<R> {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async move {
            Ok(true)
        }
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session
    ) -> Result<(), Self::Error> {
         let shell_id_opt = *self.shell_channel_id.lock().await;

         if Some(channel) == shell_id_opt {
            // 性能优先：收到一段输出就立刻 emit，避免 vim 这类交互程序因输出延迟导致光标异常跳动。
            let text = String::from_utf8_lossy(data).to_string();
            let _ = self.window.emit("ssh-output", SshPayload {
                server_id: self.server_id.clone(),
                session_id: self.session_id.clone(),
                data: text,
            });
         }
         Ok(())
    }

    async fn disconnected(&mut self, _reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        let _ = self.window.emit("ssh-closed", serde_json::json!({
            "server_id": self.server_id,
            "session_id": self.session_id,
        }));
        Ok(())
    }
}

pub struct ActiveSession {
    pub handle: Arc<Mutex<client::Handle<ClientHandler<tauri::Wry>>>>,
    pub channel_id: ChannelId,
    pub channel: Arc<Mutex<russh::Channel<russh::client::Msg>>>,
    pub sftp: Arc<Mutex<Option<SftpSession>>>,
}

pub struct SyncRuntime {
    pub debounce_active: bool,
    pub sync_in_progress: bool,
    pub pending_push: bool,
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self {
            debounce_active: false,
            sync_in_progress: false,
            pending_push: false,
        }
    }
}

pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
    pub local_sessions: LocalSessionMap,
    pub db: Arc<redb::Database>,
    pub cancelled_tasks: Arc<Mutex<HashSet<String>>>,
    pub p2p_sender: mpsc::UnboundedSender<p2p::P2PCommand>,
    pub sync_runtime: Arc<tokio::sync::Mutex<SyncRuntime>>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            local_sessions: self.local_sessions.clone(),
            db: self.db.clone(),
            cancelled_tasks: self.cancelled_tasks.clone(),
            p2p_sender: self.p2p_sender.clone(),
            sync_runtime: self.sync_runtime.clone(),
        }
    }
}

async fn authenticate<R: Runtime>(
    handle: &mut client::Handle<ClientHandler<R>>,
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

        let auth_res = handle.authenticate_publickey(&config.username, key_with_alg).await
            .map_err(|e| format!("私钥认证出错: {}", e))?;

        if !matches!(auth_res, russh::client::AuthResult::Success) {
            return Err("私钥认证被拒绝".into());
        }
    } else {
        let pass = config.password.as_deref().unwrap_or("");
        let auth_res = handle.authenticate_password(&config.username, pass).await
            .map_err(|e| format!("密码认证出错: {}", e))?;

        if !matches!(auth_res, russh::client::AuthResult::Success) {
            return Err("用户名或密码错误".into());
        }
    }
    Ok(())
}

async fn create_recursive_session<R: Runtime>(
    window: tauri::Window<R>,
    target_config: &ServerConfig,
    all_configs: &Vec<ServerConfig>,
    session_id: String,
    shell_channel_id: Arc<Mutex<Option<ChannelId>>>,
) -> Result<client::Handle<ClientHandler<R>>, String> {
    // Increase channel_buffer_size to reduce backpressure-induced stalls
    // under bursty interactive input (e.g. vim scrolling).
    let mut config = client::Config::default();
    config.channel_buffer_size = 4096;
    config.nodelay = true; // reduce latency for small packets
    let client_config = Arc::new(config);
    let connect_timeout = Duration::from_secs(10);

    let handler = ClientHandler {
        window: window.clone(),
        server_id: target_config.id.clone(),
        session_id: session_id.clone(),
        shell_channel_id: shell_channel_id.clone(),
    };

    match target_config.jump_host_id.as_deref() {
        None | Some("") => {
            let addr = format!("{}:{}", target_config.host, target_config.port);

            // ✅ 修正：使用 .await 结尾，并确保 timeout 已经导入
            let mut handle = timeout(connect_timeout, client::connect(client_config, addr, handler))
                .await
                .map_err(|_| format!("连接目标 {} 超时", target_config.host))?
                .map_err(|e| format!("直连失败: {}", e))?;

            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
        Some(jump_id) => {
            let jump_config = all_configs.iter().find(|s| s.id == jump_id)
                .ok_or(format!("找不到跳板机: {}", jump_id))?;

            let jump_handle = Box::pin(create_recursive_session(
                window.clone(),
                jump_config,
                all_configs,
                format!("{}_tunnel", session_id),
                shell_channel_id.clone()
            )).await?;
            println!("隧道已建立，正在尝试在隧道内连接目标: {}:{}", target_config.host, target_config.port);
            let channel = timeout(
                Duration::from_secs(8),
                jump_handle.channel_open_direct_tcpip(
                    &target_config.host,
                    target_config.port as u32,
                    "127.0.0.1",
                    0
                )
            ).await // ✅ 修正：.await 放在这里
            .map_err(|_| "跳板机建立隧道响应超时".to_string())?
            .map_err(|e| format!("隧道建立失败: {}", e))?;

            let mut handle = timeout(
                connect_timeout,
                client::connect_stream(client_config, channel.into_stream(), handler)
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
async fn get_servers(state: State<'_, AppState>) -> Result<Vec<ServerConfig>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

    let mut servers = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let mut server: ServerConfig = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if server.deleted {
            continue;
        }

        if !server.host.is_empty() {
            server.host = decrypt_secret(&server.host).unwrap_or_else(|_| "DECRYPT_ERROR".into());
        }
        if let Some(ref pass) = server.password {
            if !pass.is_empty() {
                server.password = Some(decrypt_secret(pass).unwrap_or_else(|_| "".into()));
            }
        }

        servers.push(server);
    }

    servers.sort_by(|a, b| {
        a.sort_order.cmp(&b.sort_order)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(servers)
}

#[tauri::command]
async fn update_server_order(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>
) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

        for (index, id) in ids.iter().enumerate() {
            // --- 核心修复：通过作用域或显式转换，确保读取动作结束 ---
            let existing_json = {
                // get 返回的 AccessGuard 在这个花括号结束时会被 drop
                table.get(id.as_str())
                    .map_err(|e| e.to_string())?
                    .map(|v| v.value().to_string()) // 转换为 Owned String
            };

            if let Some(json_str) = existing_json {
                let mut server: ServerConfig = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

                server.sort_order = index as i32;
                server.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let new_json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

                // 此时原有不可变借用已失效，可以安全地进行 mutable borrow (insert)
                table.insert(id.as_str(), new_json.as_str()).map_err(|e| e.to_string())?;
            }
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;
    schedule_push_sync(state.inner(), app_handle).await;

    Ok(())
}


#[tauri::command]
async fn get_server_latency(host: String, port: u16) -> Result<u32, String> {
    let address = format!("{}:{}", host, port);
    let start = Instant::now();

    match tokio::time::timeout(Duration::from_millis(2000), TcpStream::connect(&address)).await {
        Ok(Ok(_)) => {
            let duration = start.elapsed().as_millis() as u32;
            Ok(duration)
        }
        Ok(Err(e)) => Err(format!("连接拒绝: {}", e)),
        Err(_) => Err("连接超时".into()),
    }
}

#[tauri::command]
async fn save_server(app_handle: tauri::AppHandle, state: State<'_, AppState>, mut server: ServerConfig) -> Result<ServerConfig, String> {
    println!("\n========== [SAVE_SERVER] 开始保存 ==========");
    println!("接收到的原始数据:");
    println!("  - id: '{}'", server.id);
    println!("  - name: '{}'", server.name);

    if server.id.is_empty() {
        server.id = Uuid::new_v4().to_string();
        println!("生成新 ID: {}", server.id);
    }

    server.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    server.deleted = false;

    if !server.host.is_empty() {
        server.host = encrypt_secret(&server.host)?;
    }
    if let Some(ref pass) = server.password {
        if !pass.is_empty() {
            server.password = Some(encrypt_secret(pass)?);
        }
    }

    if let Some(ref jump_id) = server.jump_host_id {
        if jump_id.is_empty() {
            server.jump_host_id = None;
            println!("jump_host_id 转换为: None");
        }
    }

    if let Some(ref group) = server.group {
        if group.trim().is_empty() {
            server.group = None;
        } else {
            server.group = Some(group.trim().to_string());
        }
    }

    println!("\n处理后的数据:");
    println!("  - id: '{}'", server.id);
    println!("  - updated_at: {}", server.updated_at);

    // 4. 数据库持久化
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

        println!("\n序列化 JSON: {}", json);
        println!("准备插入数据库...");

        table.insert(server.id.as_str(), json.as_str()).map_err(|e| e.to_string())?;
        println!("✓ 插入成功");
    }

    match write_txn.commit() {
        Ok(_) => {
            println!("✓ 事务提交成功");
            println!("========== [SAVE_SERVER] 保存完成 ========== {}\n", server.id);
            schedule_push_sync(state.inner(), app_handle).await;
            Ok(server)
        }
        Err(e) => {
            println!("✗ 事务提交失败：{:?}", e);
            Err(e.to_string())
        }
    }
}


#[tauri::command]
async fn delete_server(app_handle: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;

    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

        let existing_data = table.get(id.as_str())
            .map_err(|e| e.to_string())?
            .map(|v| v.value().to_string());

        if let Some(json_str) = existing_data {
            let mut server: ServerConfig = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

            server.deleted = true;
            server.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

            table.insert(id.as_str(), json.as_str()).map_err(|e| e.to_string())?;
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;

    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
struct SessionWindowBootstrap {
    server_id: String,
    session_id: String,
    session_name: String,
    #[serde(default)]
    is_local: bool,
}

fn collect_servers_json(db: &Arc<redb::Database>) -> String {
    (|| -> Option<String> {
        let read_txn = db.begin_read().ok()?;
        let table = read_txn.open_table(SERVERS_TABLE).ok()?;
        let mut list = Vec::new();

        for result in table.iter().ok()? {
            if let Ok((_, value)) = result {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(value.value()) {
                    if val.get("deleted").and_then(|d| d.as_bool()) == Some(false) {
                        if let Some(host) = val.get_mut("host").and_then(|h| h.as_str()) {
                            if let Ok(decrypted) = decrypt_secret(host) {
                                val["host"] = serde_json::Value::String(decrypted);
                            }
                        }
                        list.push(val);
                    }
                }
            }
        }

        list.sort_by(|a, b| {
            let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            name_a.cmp(&name_b)
        });

        serde_json::to_string(&list).ok()
    })()
    .unwrap_or_else(|| "[]".to_string())
}

#[tauri::command]
async fn open_session_window(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    server_id: String,
    base_name: String,
) -> Result<String, String> {
    let is_local = server_id == LOCAL_SERVER_ID;
    let session_id = if is_local {
        format!("local-{}", &Uuid::new_v4().simple().to_string()[..8])
    } else {
        format!(
            "{}-{}",
            server_id,
            &Uuid::new_v4().simple().to_string()[..8]
        )
    };
    let session_name = format!("{} (窗口)", base_name.trim());
    let label = format!(
        "session-{}",
        session_id.replace(['/', '\\', ':', ' '], "-")
    );

    if app.get_webview_window(&label).is_some() {
        return Err("该会话窗口已打开".into());
    }

    let bootstrap = SessionWindowBootstrap {
        server_id: server_id.clone(),
        session_id: session_id.clone(),
        session_name: session_name.clone(),
        is_local,
    };
    let bootstrap_json = serde_json::to_string(&bootstrap).map_err(|e| e.to_string())?;
    let servers_json = collect_servers_json(&state.db);
    let init_script = format!(
        "window.__SESSION_BOOTSTRAP__ = {}; window.__INITIAL_SERVERS__ = {};",
        bootstrap_json, servers_json
    );

    let url = if cfg!(debug_assertions) {
        WebviewUrl::External(
            "http://localhost:1420"
                .parse()
                .map_err(|e| format!("无效的 dev URL: {}", e))?,
        )
    } else {
        WebviewUrl::App("index.html".into())
    };

    let window = WebviewWindowBuilder::new(&app, &label, url)
        .title(&session_name)
        .inner_size(1200.0, 800.0)
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;

    let win = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    });

    Ok(session_id)
}

#[tauri::command]
fn get_local_shell_label() -> String {
    local_shell::local_shell_label().to_string()
}

#[tauri::command]
async fn spawn_local_shell(
    window: tauri::Window,
    state: State<'_, AppState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    {
        let locals = state.local_sessions.lock().await;
        if locals.contains_key(&session_id) {
            return Ok(());
        }
    }

    local_shell::spawn_local_shell(
        window,
        session_id,
        state.local_sessions.clone(),
        cols.clamp(40, u32::from(u16::MAX)) as u16,
        rows.clamp(8, u32::from(u16::MAX)) as u16,
    )
    .await
}

#[tauri::command]
async fn connect_ssh(
    window: tauri::Window,
    state: State<'_, AppState>,
    server_id: String,
    session_id: String
) -> Result<(), String> {
    {
        let sessions = state.sessions.lock().await;
        if sessions.contains_key(&session_id) { return Ok(()); }
    }
    let shell_id_container = Arc::new(Mutex::new(None));
    let servers = get_servers(state.clone()).await?;
    let target_config = servers.iter().find(|s| s.id == server_id)
        .ok_or("配置不存在")?.clone();
    let handle = create_recursive_session(window.clone(), &target_config, &servers, session_id.clone(), shell_id_container.clone())
        .await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    {
        let mut id_lock = shell_id_container.lock().await;
        *id_lock = Some(channel.id());
    }
    channel.request_pty(true, "xterm", 80, 24, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
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
async fn write_to_ssh(state: State<'_, AppState>, session_id: String, data: String) -> Result<(), String> {
    if state.local_sessions.lock().await.contains_key(&session_id) {
        return local_shell::write_local_shell(&state.local_sessions, &session_id, data).await;
    }

    // 不要在持有全局 sessions 锁的情况下 await 网络/通道写操作。
    // 否则当输入频繁（例如 vim 上下移动不断发控制序列）时，会堆积请求导致终端假死。
    let (handle_mutex, channel_id) = {
        let sessions = state.sessions.lock().await;
        let sess = sessions.get(&session_id).ok_or("Session not found")?;
        (sess.handle.clone(), sess.channel_id)
    };

    // 给写入过程分别加超时：
    // 1) 锁等待超时：说明前一次写入还没释放 session 级 Handle 锁
    // 2) 写入超时：说明拿到锁后，handle.data(...).await 本身卡住
    let lock_timeout = Duration::from_secs(2);
    let write_timeout = Duration::from_secs(15);

    let handle = match timeout(lock_timeout, handle_mutex.lock()).await {
        Ok(guard) => guard,
        Err(_) => return Err(format!("写入 SSH 锁超时: session_id={}", session_id)),
    };

    match timeout(write_timeout, handle.data(channel_id, data.into())).await {
        Ok(res) => res.map_err(|e| format!("写入 SSH 通道失败: {:?}", e)),
        Err(_) => Err(format!("写入 SSH 写入超时: session_id={}", session_id)),
    }
}

#[tauri::command]
async fn disconnect_ssh(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
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
    Ok(())
}

#[tauri::command]
async fn resize_ssh(
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
        // 终端 UI 可能先于 SSH 连接完成初始化，此时忽略 resize 即可
        Ok(())
    }
}

#[tauri::command]
async fn list_local_dir(path: String) -> Result<Vec<FileInfo>, String> {
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    files.push(FileInfo { name: "..".to_string(), is_dir: true, size: 0 });
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(meta) = entry.metadata() {
                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_dir: meta.is_dir(),
                    size: meta.len(),
                });
            }
        }
    }
    files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(files)
}

fn format_rwx(mode: u32) -> String {
    let mode = mode & 0o777;
    let chars = ["---", "--x", "-w-", "-wx", "r--", "r-x", "rw-", "rwx"];
    format!(
        "{}{}{}",
        chars[((mode >> 6) & 7) as usize],
        chars[((mode >> 3) & 7) as usize],
        chars[(mode & 7) as usize]
    )
}

fn parse_octal_mode(mode: &str) -> Result<u32, String> {
    let trimmed = mode.trim();
    let digits = trimmed.strip_prefix('0').unwrap_or(trimmed);
    if digits.is_empty() || digits.len() > 4 || !digits.chars().all(|c| c.is_ascii_digit() && c < '8') {
        return Err("无效的权限格式，请使用八进制数如 755 或 0644".into());
    }
    u32::from_str_radix(digits, 8).map_err(|_| "无效的权限格式，请使用八进制数如 755 或 0644".into())
}

fn file_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn build_file_detail(
    path: String,
    is_dir: bool,
    size: u64,
    permissions: Option<u32>,
    modified_at: Option<u64>,
    accessed_at: Option<u64>,
    uid: Option<u32>,
    gid: Option<u32>,
    user: Option<String>,
    group: Option<String>,
) -> FileDetail {
    let permissions_text = permissions.map(format_rwx);
    let permissions = permissions.map(|mode| format!("{:o}", mode & 0o777));
    FileDetail {
        name: file_name_from_path(&path),
        path,
        is_dir,
        size,
        permissions,
        permissions_text,
        modified_at,
        accessed_at,
        uid,
        gid,
        user,
        group,
    }
}

async fn open_sftp_session(state: &AppState, session_id: &str) -> Result<SftpSession, String> {
    let handle_mutex = {
        let sessions = state.sessions.lock().await;
        let sess = sessions.get(session_id).ok_or("Session not found")?;
        sess.handle.clone()
    };

    let channel = {
        let handle = handle_mutex.lock().await;
        handle
            .channel_open_session()
            .await
            .map_err(|e| e.to_string())?
    };
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;

    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("SFTP 初始化失败: {}", e))
}

#[tauri::command]
async fn get_local_file_info(path: String) -> Result<FileDetail, String> {
    let meta = tokio::fs::metadata(&path).await.map_err(|e| e.to_string())?;
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let accessed_at = meta
        .accessed()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        Some(meta.permissions().mode())
    };
    #[cfg(not(unix))]
    let permissions: Option<u32> = None;

    Ok(build_file_detail(
        path,
        meta.is_dir(),
        meta.len(),
        permissions,
        modified_at,
        accessed_at,
        None,
        None,
        None,
        None,
    ))
}

#[tauri::command]
async fn rename_local_file(old_path: String, new_path: String) -> Result<(), String> {
    tokio::fs::rename(&old_path, &new_path)
        .await
        .map_err(|e| format!("重命名失败: {}", e))
}

#[tauri::command]
async fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("路径不存在".into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if path.is_dir() {
            Command::new("explorer")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        } else {
            let selected = path
                .to_str()
                .ok_or_else(|| "路径包含无效字符".to_string())?;
            Command::new("explorer")
                .args(["/select,", selected])
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let path_str = path
            .to_str()
            .ok_or_else(|| "路径包含无效字符".to_string())?;
        if path.is_file() {
            Command::new("open")
                .args(["-R", path_str])
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        } else {
            Command::new("open")
                .arg(path_str)
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let target = if path.is_file() {
            path.parent().unwrap_or(path.as_path())
        } else {
            path.as_path()
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn get_remote_file_info(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<FileDetail, String> {
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    let meta = sftp.metadata(&path).await.map_err(|e| e.to_string())?;
    Ok(build_file_detail(
        path,
        meta.is_dir(),
        meta.len(),
        meta.permissions,
        meta.mtime.map(u64::from),
        meta.atime.map(u64::from),
        meta.uid,
        meta.gid,
        meta.user,
        meta.group,
    ))
}

#[tauri::command]
async fn rename_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    sftp.rename(old_path, new_path)
        .await
        .map_err(|e| format!("重命名失败: {}", e))
}

#[tauri::command]
async fn set_remote_file_permissions(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    mode: String,
) -> Result<(), String> {
    let parsed_mode = parse_octal_mode(&mode)?;
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    let mut meta = sftp.metadata(&path).await.map_err(|e| e.to_string())?;
    let current = meta.permissions.unwrap_or(if meta.is_dir() { 0o040755 } else { 0o100644 });
    let type_bits = current & !0o777;
    meta.permissions = Some(type_bits | (parsed_mode & 0o777));
    sftp.set_metadata(&path, meta)
        .await
        .map_err(|e| format!("修改权限失败: {}", e))
}

#[tauri::command]
async fn delete_local_file(path: String, is_dir: bool) -> Result<(), String> {
    if is_dir {
        tokio::fs::remove_dir(&path)
            .await
            .map_err(|e| format!("删除目录失败: {}", e))
    } else {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("删除文件失败: {}", e))
    }
}

async fn copy_dir_all(src: &Path, dest: &Path) -> Result<(), String> {
    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| format!("创建目录失败: {}", e))?;
    let mut entries = tokio::fs::read_dir(src)
        .await
        .map_err(|e| format!("读取目录失败: {}", e))?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("读取文件类型失败: {}", e))?;
        if file_type.is_dir() {
            Box::pin(copy_dir_all(&entry_path, &dest_path)).await?;
        } else {
            tokio::fs::copy(&entry_path, &dest_path)
                .await
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn create_local_path(path: String, is_dir: bool) -> Result<(), String> {
    if Path::new(&path).exists() {
        return Err("路径已存在".into());
    }
    if is_dir {
        tokio::fs::create_dir(&path)
            .await
            .map_err(|e| format!("创建文件夹失败: {}", e))
    } else {
        if let Some(parent) = Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建父目录失败: {}", e))?;
        }
        tokio::fs::File::create(&path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
async fn copy_local_path(src: String, dest: String) -> Result<(), String> {
    let src_path = Path::new(&src);
    if !src_path.exists() {
        return Err("源路径不存在".into());
    }
    if Path::new(&dest).exists() {
        return Err("目标已存在".into());
    }
    if src_path.is_dir() {
        copy_dir_all(src_path, Path::new(&dest)).await
    } else {
        if let Some(parent) = Path::new(&dest).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建父目录失败: {}", e))?;
        }
        tokio::fs::copy(&src, &dest)
            .await
            .map_err(|e| format!("复制失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
async fn move_local_path(src: String, dest: String, is_dir: bool) -> Result<(), String> {
    if Path::new(&dest).exists() {
        return Err("目标已存在".into());
    }
    if let Some(parent) = Path::new(&dest).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    match tokio::fs::rename(&src, &dest).await {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_local_path(src.clone(), dest.clone()).await?;
            if is_dir {
                tokio::fs::remove_dir_all(&src)
                    .await
                    .map_err(|e| format!("移动失败: {}", e))
            } else {
                tokio::fs::remove_file(&src)
                    .await
                    .map_err(|e| format!("移动失败: {}", e))
            }
        }
    }
}

async fn copy_remote_file_stream(sftp: &SftpSession, src: &str, dest: &str) -> Result<(), String> {
    let mut src_file = sftp.open(src).await.map_err(|e| e.to_string())?;
    let mut dest_file = sftp.create(dest).await.map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 65536];
    loop {
        let n = src_file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        dest_file
            .write_all(&buffer[..n])
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn copy_remote_recursive(sftp: &SftpSession, src: &str, dest: &str) -> Result<(), String> {
    let meta = sftp.metadata(src).await.map_err(|e| e.to_string())?;
    if meta.is_dir() {
        sftp.create_dir(dest).await.map_err(|e| e.to_string())?;
        for entry in sftp.read_dir(src).await.map_err(|e| e.to_string())? {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let sub_src = format!("{}/{}", src.trim_end_matches('/'), name);
            let sub_dest = format!("{}/{}", dest.trim_end_matches('/'), name);
            Box::pin(copy_remote_recursive(sftp, &sub_src, &sub_dest)).await?;
        }
    } else {
        copy_remote_file_stream(sftp, src, dest).await?;
    }
    Ok(())
}

#[tauri::command]
async fn create_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    if is_dir {
        sftp.create_dir(&path)
            .await
            .map_err(|e| format!("创建文件夹失败: {}", e))
    } else {
        sftp.create(&path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
async fn copy_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    src: String,
    dest: String,
) -> Result<(), String> {
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    copy_remote_recursive(&sftp, &src, &dest).await
}

#[tauri::command]
async fn move_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    src: String,
    dest: String,
) -> Result<(), String> {
    let sftp = open_sftp_session(state.inner(), &session_id).await?;
    sftp.rename(src, dest)
        .await
        .map_err(|e| format!("移动失败: {}", e))
}

#[tauri::command]
async fn list_remote_dir(state: State<'_, AppState>, session_id: String, path: String) -> Result<Vec<FileInfo>, String> {
    let handle_mutex = {
        let sessions = state.sessions.lock().await;
        let sess = sessions.get(&session_id).ok_or("Session not found")?;
        sess.handle.clone()
    };

    let channel = {
        let handle = handle_mutex.lock().await;
        handle
            .channel_open_session()
            .await
            .map_err(|e| e.to_string())?
    };
    channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;

    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("SFTP 初始化失败: {}", e))?;

    let entries = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    files.push(FileInfo { name: "..".to_string(), is_dir: true, size: 0 });

    for entry in entries {
        let filename = entry.file_name();
        if filename == "." || filename == ".." { continue; }
        let metadata = entry.metadata();
        files.push(FileInfo {
            name: filename.to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.size.unwrap_or(0),
        });
    }
    files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(files)
}



#[tauri::command]
async fn sftp_upload(
    window: tauri::Window,
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String
) -> Result<(), String> {
    let sftp = {
        let handle_mutex = {
            let sessions = state.sessions.lock().await;
            let sess = sessions.get(&session_id).ok_or("Session not found")?;
            sess.handle.clone()
        };

        let channel = {
            let handle = handle_mutex.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("打开通道失败: {:?}", e))?
        };
        channel.request_subsystem(true, "sftp").await.map_err(|e| format!("请求子系统失败: {:?}", e))?;

        SftpSession::new(channel.into_stream()).await.map_err(|e| format!("初始化 SFTP 失败: {:?}", e))?
    };

    let mut local_file = tokio::fs::File::open(&local_path).await.map_err(|e| e.to_string())?;
    let total_size = local_file.metadata().await.map_err(|e| e.to_string())?.len();

    let mut remote_file = sftp.create(&remote_path).await.map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; 65536];
    let mut uploaded_size = 0u64;

    while let Ok(n) = local_file.read(&mut buffer).await {
        if n == 0 { break; }

        if state.cancelled_tasks.lock().await.contains(&task_id) {
            state.cancelled_tasks.lock().await.remove(&task_id);
            drop(remote_file);
            return Err("Task cancelled".into());
        }

        remote_file.write_all(&buffer[..n]).await.map_err(|e| e.to_string())?;
        uploaded_size += n as u64;

        let progress = ((uploaded_size as f64 / total_size as f64) * 100.0) as u64;
        let _ = window.emit("transfer-progress", ProgressPayload {
            task_id: task_id.clone(),
            progress
        });
    }
    Ok(())
}

#[tauri::command]
async fn sftp_download(
    window: tauri::Window,
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String
) -> Result<(), String> {
    let sftp = {
        let handle_mutex = {
            let sessions = state.sessions.lock().await;
            let sess = sessions.get(&session_id).ok_or("Session not found")?;
            sess.handle.clone()
        };

        let channel = {
            let handle = handle_mutex.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("{:?}", e))?
        };
        channel.request_subsystem(true, "sftp").await.map_err(|e| format!("{:?}", e))?;

        SftpSession::new(channel.into_stream()).await.map_err(|e| format!("{:?}", e))?
    };

    let mut remote_file = sftp.open(&remote_path).await.map_err(|e| e.to_string())?;
    let metadata = remote_file.metadata().await.map_err(|e| e.to_string())?;
    let total_size = metadata.size.unwrap_or(0);

    let mut local_file = tokio::fs::File::create(&local_path).await.map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; 65536];
    let mut downloaded_size = 0u64;

    while let Ok(n) = remote_file.read(&mut buffer).await {
        if n == 0 { break; }

        if state.cancelled_tasks.lock().await.contains(&task_id) {
            state.cancelled_tasks.lock().await.remove(&task_id);
            drop(remote_file);
            return Err("Task cancelled".into());
        }

        local_file.write_all(&buffer[..n]).await.map_err(|e| e.to_string())?;
        downloaded_size += n as u64;

        if total_size > 0 {
            let progress = ((downloaded_size as f64 / total_size as f64) * 100.0) as u64;
            let _ = window.emit("transfer-progress", ProgressPayload {
                task_id: task_id.clone(),
                progress
            });
        }
    }

    let _ = window.emit("transfer-progress", ProgressPayload {
        task_id: task_id.clone(),
        progress: 100
    });

    Ok(())
}

#[tauri::command]
async fn abort_transfer(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.cancelled_tasks.lock().await.insert(task_id);
    Ok(())
}

#[tauri::command]
async fn delete_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    is_dir: bool
) -> Result<(), String> {
    let sftp = {
        let handle_mutex = {
            let sessions = state.sessions.lock().await;
            let sess = sessions.get(&session_id).ok_or("Session not found")?;
            sess.handle.clone()
        };

        let ch = {
            let handle = handle_mutex.lock().await;
            handle.channel_open_session().await.map_err(|e| e.to_string())?
        };
        ch.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
        SftpSession::new(ch.into_stream()).await.map_err(|e| e.to_string())?
    };

    if is_dir {
        sftp.remove_dir(path).await.map_err(|e| format!("删除目录失败: {}", e))
    } else {
        sftp.remove_file(path).await.map_err(|e| format!("删除文件失败: {}", e))
    }
}

#[tauri::command]
async fn get_quick_commands(state: State<'_, AppState>) -> Result<Vec<QuickCommand>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;

    let mut commands = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let cmd: QuickCommand = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if !cmd.deleted {
            commands.push(cmd);
        }
    }

    commands.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(commands)
}

#[tauri::command]
async fn save_quick_command(app_handle: tauri::AppHandle, state: State<'_, AppState>, mut cmd: QuickCommand) -> Result<QuickCommand, String> {
    if cmd.id.is_empty() {
        cmd.id = Uuid::new_v4().to_string();
    }

    cmd.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    cmd.deleted = false;

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;

        table.insert(cmd.id.as_str(), json.as_str()).map_err(|e| e.to_string())?;
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;

    Ok(cmd)
}

#[tauri::command]
async fn delete_quick_command(app_handle: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;

    {
        let mut table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;

        let existing_data = table.get(id.as_str())
            .map_err(|e| e.to_string())?
            .map(|v| v.value().to_string());

        if let Some(json_str) = existing_data {
            let mut cmd: QuickCommand = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

            cmd.deleted = true;
            cmd.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let json = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;

            table.insert(id.as_str(), json.as_str()).map_err(|e| e.to_string())?;
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    // 异步触发同步逻辑
    schedule_push_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
async fn save_ai_config(app_handle: tauri::AppHandle, state: State<'_, AppState>, mut config: AiConfig) -> Result<(), String> {
    if !config.api_key.is_empty() {
        config.api_key = encrypt_secret(&config.api_key)?;
    }
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table.insert("default", json.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    // 触发同步
    schedule_push_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
async fn get_ai_config(state: State<'_, AppState>) -> Result<Option<AiConfig>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;

    if let Some(value) = table.get("default").map_err(|e| e.to_string())? {
        let mut config: AiConfig = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        if !config.api_key.is_empty() {
            config.api_key = decrypt_secret(&config.api_key).unwrap_or_else(|_| "".into());
        }
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn ask_ai(
    window: Window,
    prompt: String,
    config: AiConfig,
    task_id: String,
) -> Result<(), String> {
    let url = match config.current_provider.as_str() {
        "deepseek" => "https://api.deepseek.com/v1/chat/completions",
        "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
        "doubao" => "https://ark.cn-beijing.volces.com/api/v3/chat/completions",
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        _ => "https://api.openai.com/v1/chat/completions",
    };

    let client = reqwest::Client::new();

    let body = json!({
        "model": config.model,
        "messages": [
            {
                "role": "system",
                "content": "你是一个 Linux 专家和 SSH 终端助手。请给出专业、简洁的回答，命令请使用 Markdown 代码块包裹。"
            },
            {"role": "user", "content": prompt}
        ],
        "stream": true,
        "temperature": config.temperature
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let _ = window.emit("ai-res-chunk", json!({ "taskId": &task_id, "content": format!("❌ 网络请求失败: {}", e) }));
            e.to_string()
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_else(|_| "无法读取错误详情".into());
        let detailed_err = if let Ok(json_err) = serde_json::from_str::<serde_json::Value>(&err_text) {
            json_err["error"]["message"].as_str().unwrap_or(&err_text).to_string()
        } else {
            err_text
        };
        let final_err = format!("API 错误 ({}): {}", status, detailed_err);
        let _ = window.emit("ai-res-chunk", json!({ "taskId": &task_id, "content": format!("❌ {}", final_err) }));
        return Err(final_err);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer.drain(..line_end + 1).collect::<String>();
                    let line = line.trim();

                    if line.is_empty() { continue; }
                    if line == "data: [DONE]" {
                        return Ok(());
                    }

                    if let Some(data_json) = line.strip_prefix("data: ") {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_json) {
                            if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                                window.emit("ai-res-chunk", json!({
                                    "taskId": &task_id,
                                    "content": content
                                })).map_err(|e| e.to_string())?;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = window.emit("ai-res-chunk", json!({ "taskId": &task_id, "content": format!("\n[流传输中断: {}]", e) }));
                return Err(e.to_string());
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HttpRequestPayload {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HttpResponsePayload {
    status: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: String,
    elapsed_ms: u64,
}

#[tauri::command]
async fn send_http_request(payload: HttpRequestPayload) -> Result<HttpResponsePayload, String> {
    let method = reqwest::Method::from_bytes(payload.method.as_bytes())
        .map_err(|e| format!("无效的 HTTP 方法: {}", e))?;

    let timeout = Duration::from_millis(payload.timeout_ms.unwrap_or(30_000));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut request = client.request(method, &payload.url);
    for (key, value) in payload.headers {
        if !key.is_empty() {
            request = request.header(key, value);
        }
    }
    if let Some(body) = payload.body.filter(|b| !b.is_empty()) {
        request = request.body(body);
    }

    let started = Instant::now();
    let response = request.send().await.map_err(|e| format!("请求失败: {}", e))?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    let body = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    Ok(HttpResponsePayload {
        status: status.as_u16(),
        status_text: status
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string(),
        headers,
        body,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
async fn send_p2p_message(
    state: tauri::State<'_, AppState>,
    target: String,
    content: String,
) -> Result<(), String> {
    println!("[Command] 准备发送消息到 P2P 队列: target={}, content={}", target, content);

    state.p2p_sender.send(p2p::P2PCommand::SendMessage { target, content })
        .map_err(|e| {
            let err_msg = format!("发送到后台任务失败: {}", e);
            eprintln!("[Command] 错误: {}", err_msg);
            err_msg
        })?;

    println!("[Command] 消息已成功推入异步队列");
    Ok(())
}

#[tauri::command]
async fn send_p2p_file(
    state: State<'_, AppState>,
    target: String,
    path: String,
) -> Result<(), String> {
    let path_buf = std::path::PathBuf::from(path);
    // 检查文件是否存在
    if !path_buf.exists() {
        return Err("文件不存在".into());
    }

    state.p2p_sender.send(p2p::P2PCommand::SendFile {
        target,
        path: path_buf
    }).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_p2p_messages(
    state: tauri::State<'_, AppState>,
    peer_id: String,
) -> Result<Vec<p2p::ChatMessageRecord>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(P2P_MESSAGES_TABLE).map_err(|e| e.to_string())?;

    let mut msgs = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let msg: p2p::ChatMessageRecord = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        // 筛选与该 Peer 的对话
        if msg.peer_id == peer_id {
            msgs.push(msg);
        }
    }

    // 按时间戳从旧到新排序
    msgs.sort_by_key(|m| m.timestamp);
    Ok(msgs)
}


pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // 1. 获取应用数据目录
            let app_data_dir = handle.path().app_data_dir().expect("无法获取应用数据目录");
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("无法创建目录");
            }
            let db_path = app_data_dir.join("hiphup_ssh_v1.redb");

            // 2. 快速打开数据库
            let db = Database::builder()
                .create(db_path)
                .expect("无法打开数据库");
            let db_arc = Arc::new(db);

            if let Some(main_window) = app.get_webview_window("main") {
                preheat_servers(&main_window, &db_arc);
            }

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

            let shared_p2p_status = Arc::new(p2p::P2PStatus {
                online_peers: std::sync::Mutex::new(std::collections::HashSet::new()),
            });

            // 3. 注入状态 (使用标准库 Mutex)
            app.manage(AppState {
                sessions: Arc::new(Mutex::new(HashMap::new())),
                local_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                db: db_arc.clone(),
                cancelled_tasks: Arc::new(Mutex::new(HashSet::new())),
                p2p_sender: tx,
                sync_runtime: Arc::new(tokio::sync::Mutex::new(SyncRuntime::default())),
            });

           app.manage(shared_p2p_status.clone());

           let handle_for_startup = app.handle().clone();
           tauri::async_runtime::spawn(async move {
               tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
               if let Some(state) = handle_for_startup.try_state::<AppState>() {
                   run_startup_pull(state.inner(), handle_for_startup.clone()).await;
               }
           });

           // 💡 4. 启动 P2P 节点，传入 handle 和同一个 status 实例
           let handle_for_p2p = app.handle().clone();
           let status_for_node = shared_p2p_status.clone(); // 指向同一块内存
           let db_for_p2p = db_arc.clone();

           tauri::async_runtime::spawn(async move {
               // 确保这里的参数顺序和 p2p.rs 定义的一致
               let _ = start_p2p_node(handle_for_p2p, rx, db_for_p2p, status_for_node).await;
           });

            app.manage(redis_manager::RedisState {
                connection: Arc::new(tokio::sync::Mutex::new(None)),
            });

            if let Some(main_window) = app.get_webview_window("main") {
                let win = main_window.clone();
                tauri::async_runtime::spawn(async move {
                    // 给予 WebView 渲染 HTML 背景的时间 (150-200ms 足够)
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                    // 暴力夺取焦点三部曲：
                    let _ = win.show();             // 显示
                    let _ = win.unminimize();       // 取消最小化
                    let _ = win.set_always_on_top(true); // 强行置顶（绕过 Windows 焦点保护）
                    let _ = win.set_focus();        // 获取焦点
                    let _ = win.set_always_on_top(false); // 恢复正常层级
                });
            }

            // 4. 异步初始化和清理
            let db_for_setup = db_arc.clone();

            tauri::async_runtime::spawn(async move {
                // 稍微延迟，确保窗口已弹出
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                // A. 数据库表初始化
                let init_res = (|| -> Result<(), String> {
                    let write_txn = db_for_setup.begin_write().map_err(|e| e.to_string())?;
                    {
                        let _ = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(REDIS_CONN_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(P2P_MESSAGES_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(P2P_REMARKS_TABLE).map_err(|e| e.to_string())?;
                        let _ = write_txn.open_table(API_DEBUGGER_TABLE).map_err(|e| e.to_string())?;
                    }
                    write_txn.commit().map_err(|e| e.to_string())?;
                    Ok(())
                })();

                if let Err(e) = init_res {
                    eprintln!("[DB Error] 初始化失败: {}", e);
                }
            });

            // 5. 托盘初始化
            if let Err(e) = setup_tray(app) {
                eprintln!("托盘初始化失败: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_local_shell_label,
            spawn_local_shell,
            open_session_window,
            connect_ssh,
            resize_ssh,
            disconnect_ssh,
            write_to_ssh,
            list_local_dir,
            get_local_file_info,
            rename_local_file,
            reveal_in_file_manager,
            create_local_path,
            copy_local_path,
            move_local_path,
            delete_local_file,
            list_remote_dir,
            get_remote_file_info,
            rename_remote_file,
            set_remote_file_permissions,
            create_remote_path,
            copy_remote_path,
            move_remote_path,
            sftp_upload,
            sftp_download,
            abort_transfer,
            delete_remote_file,
            get_quick_commands,
            save_quick_command,
            delete_quick_command,
            save_ai_config,
            get_ai_config,
            ask_ai,
            get_server_latency,
            get_servers,
            update_server_order,
            save_server,
            delete_server,
            sync_to_cloud,
            sync_from_cloud,
            get_sync_settings,
            save_sync_settings,
            redis_connect,
            redis_get_keys,
            redis_get_value,
            redis_set_value,
            redis_del_key,
            redis_rename_key,
            redis_get_ttl,
            redis_get_type,
            save_redis_config,
            get_redis_configs,
            delete_redis_config,
            clear_all_redis_configs,
            send_p2p_message,
            send_p2p_file,
            get_p2p_messages,
            set_p2p_remark,
            get_p2p_remarks,
            search_p2p_messages,
            get_online_peers,
            send_http_request,
            get_api_debugger_data,
            save_api_debugger_data,
            export_api_debugger_file,
            import_api_debugger_file
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 运行出错");
}

fn setup_tray<R: tauri::Runtime>(app: &tauri::App<R>) -> Result<(), Box<dyn std::error::Error>> {
    let quit_i = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// 💡 提取出的数据预热方法
fn preheat_servers(window: &tauri::WebviewWindow, db: &Arc<redb::Database>) {
    let servers_json = collect_servers_json(db);
    let _ = window.eval(&format!("window.__INITIAL_SERVERS__ = {};", servers_json));
}