mod sync;
mod p2p;
mod security;
mod redis_manager;
use crate::sync::{
    get_sync_settings,
    save_sync_settings,
    sync_to_cloud,
    sync_from_cloud,
    trigger_auto_sync
};
use security::{encrypt_secret, decrypt_secret};
use p2p::{set_p2p_remark, start_p2p_node, get_p2p_remarks, search_p2p_messages, get_online_peers};
use redis_manager::{redis_connect, redis_get_keys, redis_get_value, redis_set_value, redis_del_key, redis_rename_key, redis_get_ttl, redis_get_type, save_redis_config, get_redis_configs, delete_redis_config, clear_all_redis_configs};
use async_trait::async_trait;
use tokio::sync::mpsc;
use russh::*;
use russh::client::DisconnectReason;
use std::sync::Arc;
use tauri::{Emitter, Window, Runtime, State, Manager};
use tokio::sync::Mutex;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use redb::{Database, TableDefinition, ReadableTable};
use uuid::Uuid;
use std::fs;
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
    pub updated_at: u64,
    #[serde(default)]
    pub deleted: bool,
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

#[async_trait]
impl<R: Runtime> client::Handler for ClientHandler<R> {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn data(&mut self, channel: ChannelId, data: &[u8], _session: &mut client::Session) -> Result<(), Self::Error> {
         let shell_id_opt = *self.shell_channel_id.lock().await;

         if Some(channel) == shell_id_opt {
             let text = String::from_utf8_lossy(data).to_string();
             let _ = self.window.emit("ssh-output", SshPayload {
                 server_id: self.server_id.clone(),
                 session_id: self.session_id.clone(),
                 data: text,
             });
         } else {
             println!("[SSH] 拦截到非 Shell 通道数据 (ID: {:?})", channel);
         }
         Ok(())
    }

    async fn disconnected(&mut self, _reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        let _ = self.window.emit("ssh-closed", serde_json::json!({ "server_id": self.server_id }));
        Ok(())
    }
}

pub struct ActiveSession {
    pub handle: client::Handle<ClientHandler<tauri::Wry>>,
    pub channel_id: ChannelId,
    pub sftp: Arc<Mutex<Option<SftpSession>>>,
}

pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
    pub db: Arc<redb::Database>,
    pub cancelled_tasks: Arc<Mutex<HashSet<String>>>,
    // 💡 新增：P2P 发送句柄
    pub p2p_sender: mpsc::UnboundedSender<p2p::P2PCommand>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            db: self.db.clone(),
            cancelled_tasks: self.cancelled_tasks.clone(),
            p2p_sender: self.p2p_sender.clone(),
        }
    }
}

async fn authenticate<R: Runtime>(
    handle: &mut client::Handle<ClientHandler<R>>,
    config: &ServerConfig,
) -> Result<(), String> {
    if config.auth_type == "key" {
        let path_str = config.private_key_path.as_ref().ok_or("未配置私钥路径")?;
        let key_pair = russh_keys::load_secret_key(path_str, None)
            .map_err(|e| format!("加载私钥失败: {}", e))?;
        let auth_res = handle.authenticate_publickey(&config.username, Arc::new(key_pair)).await
            .map_err(|e| format!("私钥认证出错: {}", e))?;
        if !auth_res { return Err("私钥认证被拒绝".into()); }
    } else {
        let pass = config.password.as_deref().unwrap_or("");
        let auth_res = handle.authenticate_password(&config.username, pass).await
            .map_err(|e| format!("密码认证出错: {}", e))?;
        if !auth_res { return Err("用户名或密码错误".into()); }
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
    let client_config = Arc::new(client::Config::default());
    let handler = ClientHandler {
        window: window.clone(),
        server_id: target_config.id.clone(),
        session_id: session_id.clone(),
        shell_channel_id: shell_channel_id.clone(),
    };

    println!("[SSH] 尝试连接 - 目标服务器：{} ({}:{})",
             target_config.name, target_config.host, target_config.port);
    println!("[SSH] jump_host_id: {:?}", target_config.jump_host_id);

    match target_config.jump_host_id.as_deref() {
        None | Some("") => {
            println!("[SSH] 直连模式");
            let addr = format!("{}:{}", target_config.host, target_config.port);
            let mut handle = client::connect(client_config, addr, handler).await
                .map_err(|e| format!("连接 {} 失败：{}", target_config.host, e))?;
            authenticate(&mut handle, target_config).await?;
            Ok(handle)
        }
        Some(jump_id) => {
            println!("[SSH] 隧道模式 - 跳板机 ID: {}", jump_id);

            let jump_config = all_configs.iter().find(|s| s.id == jump_id)
                .ok_or(format!("找不到跳板机配置：{}", jump_id))?;

            println!("[SSH] 跳板机信息：{} ({}:{})",
                     jump_config.name, jump_config.host, jump_config.port);

            let jump_handle = Box::pin(create_recursive_session(
                window.clone(),
                jump_config,
                all_configs,
                format!("{}_tunnel", session_id),
                shell_channel_id.clone()
            )).await?;

            println!("[SSH] 跳板机连接成功，尝试建立隧道到 {}:{}",
                     target_config.host, target_config.port);

            let channel = jump_handle.channel_open_direct_tcpip(
                &target_config.host,
                target_config.port as u32,
                "127.0.0.1",
                0
            ).await
            .map_err(|e| {
                println!("[SSH] 隧道建立失败：{:?}", e);
                format!("隧道建立失败：{:?}", e)
            })?;

            println!("[SSH] 隧道建立成功");

            let mut handle = client::connect_stream(client_config, channel.into_stream(), handler).await
                .map_err(|e| format!("隧道内连接失败：{}", e))?;
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

    servers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(servers)
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
            trigger_auto_sync(state.inner(), app_handle).await;
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

    trigger_auto_sync(state.inner(), app_handle).await;

    Ok(())
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
    let handle = create_recursive_session(window.clone(), &target_config, &servers, session_id.clone(), shell_id_container.clone()).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    {
        let mut id_lock = shell_id_container.lock().await;
        *id_lock = Some(channel.id());
    }
    channel.request_pty(true, "xterm", 80, 24, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
    let channel_id = channel.id();
    state.sessions.lock().await.insert(session_id, ActiveSession { handle, channel_id, sftp: Arc::new(Mutex::new(None)) });
    Ok(())
}

#[tauri::command]
async fn write_to_ssh(state: State<'_, AppState>, session_id: String, data: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    if let Some(sess) = sessions.get_mut(&session_id) {
        let mut crypto_data = russh::CryptoVec::new();
        crypto_data.extend(data.as_bytes());
        sess.handle.data(sess.channel_id, crypto_data).await
            .map_err(|_| "Failed to write data to SSH channel".to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn disconnect_ssh(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.remove(&session_id) {
        let _ = session.handle.disconnect(
            russh::Disconnect::ByApplication,
            "User closed connection",
            "en"
        ).await;
        drop(session);
    }
    Ok(())
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

#[tauri::command]
async fn list_remote_dir(state: State<'_, AppState>, session_id: String, path: String) -> Result<Vec<FileInfo>, String> {
    let mut sessions = state.sessions.lock().await;
    let sess = sessions.get_mut(&session_id).ok_or("Session not found")?;
    let channel = sess.handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(channel.into_stream()).await.map_err(|e| e.to_string())?;
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
        let mut sessions = state.sessions.lock().await;
        let sess = sessions.get_mut(&session_id).ok_or("Session not found")?;
        let channel = sess.handle.channel_open_session().await.map_err(|e| e.to_string())?;
        channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
        SftpSession::new(channel.into_stream()).await.map_err(|e| e.to_string())?
    };

    let mut local_file = tokio::fs::File::open(&local_path).await.map_err(|e| e.to_string())?;
    let total_size = local_file.metadata().await.map_err(|e| e.to_string())?.len();
    let mut remote_file = sftp.create(&remote_path).await.map_err(|e| e.to_string())?;

    let mut buffer = [0u8; 32768];
    let mut uploaded_size = 0u64;

    while let Ok(n) = local_file.read(&mut buffer).await {
        if n == 0 { break; }

        if state.cancelled_tasks.lock().await.contains(&task_id) {
            state.cancelled_tasks.lock().await.remove(&task_id);
            return Err("Task cancelled".into());
        }

        remote_file.write_all(&buffer[..n]).await.map_err(|e| e.to_string())?;
        uploaded_size += n as u64;
        let _ = window.emit("transfer-progress", ProgressPayload { task_id: task_id.clone(), progress: (uploaded_size * 100 / total_size) });
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
        let mut sessions = state.sessions.lock().await;
        let sess = sessions.get_mut(&session_id).ok_or("Session not found")?;
        let channel = sess.handle.channel_open_session().await.map_err(|e| e.to_string())?;
        channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
        SftpSession::new(channel.into_stream()).await.map_err(|e| e.to_string())?
    };

    let mut remote_file = sftp.open(&remote_path).await.map_err(|e| e.to_string())?;
    let total_size = remote_file.metadata().await.map_err(|e| e.to_string())?.size.unwrap_or(0);
    let mut local_file = tokio::fs::File::create(&local_path).await.map_err(|e| e.to_string())?;

    let mut buffer = [0u8; 32768];
    let mut downloaded_size = 0u64;

    while let Ok(n) = remote_file.read(&mut buffer).await {
        if n == 0 { break; }

        if state.cancelled_tasks.lock().await.contains(&task_id) {
            state.cancelled_tasks.lock().await.remove(&task_id);
            return Err("Task cancelled".into());
        }

        local_file.write_all(&buffer[..n]).await.map_err(|e| e.to_string())?;
        downloaded_size += n as u64;
        if total_size > 0 {
            let _ = window.emit("transfer-progress", ProgressPayload { task_id: task_id.clone(), progress: (downloaded_size * 100 / total_size) });
        }
    }
    let _ = window.emit("transfer-progress", ProgressPayload { task_id: task_id.clone(), progress: 100 });
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
        let mut sessions = state.sessions.lock().await;
        let sess = sessions.get_mut(&session_id).ok_or("Session not found")?;
        let ch = sess.handle.channel_open_session().await.map_err(|e| e.to_string())?;
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

    trigger_auto_sync(state.inner(), app_handle).await;

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
    trigger_auto_sync(state.inner(), app_handle).await;
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
    trigger_auto_sync(state.inner(), app_handle).await;
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
                db: db_arc.clone(),
                cancelled_tasks: Arc::new(Mutex::new(HashSet::new())),
                p2p_sender: tx,
            });

           app.manage(shared_p2p_status.clone());

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
            connect_ssh,
            disconnect_ssh,
            write_to_ssh,
            list_local_dir,
            list_remote_dir,
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
            get_online_peers
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
    let servers_json = (|| -> Option<String> {
        let read_txn = db.begin_read().ok()?;
        let table = read_txn.open_table(SERVERS_TABLE).ok()?;
        let mut list = Vec::new();

        for result in table.iter().ok()? {
            if let Ok((_, value)) = result {
                // 使用 from_str 解决之前的类型匹配问题
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(value.value()) {
                    // 1. 过滤已删除
                    if val.get("deleted").and_then(|d| d.as_bool()) == Some(false) {
                        // 2. 预热时解密 Host (防止界面闪烁)
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

        // 3. 统一排序逻辑 (A-Z)
        list.sort_by(|a, b| {
            let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            name_a.cmp(&name_b)
        });

        serde_json::to_string(&list).ok()
    })().unwrap_or_else(|| "[]".to_string());

    // 注入 JS
    let _ = window.eval(&format!("window.__INITIAL_SERVERS__ = {};", servers_json));
}