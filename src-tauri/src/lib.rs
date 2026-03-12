mod sync;
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
use redis_manager::{redis_connect, redis_get_keys, redis_get_value, redis_set_value, redis_del_key, redis_rename_key, redis_get_ttl, redis_get_type, save_redis_config, get_redis_configs, delete_redis_config, clear_all_redis_configs};
use async_trait::async_trait;
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
use futures::StreamExt; // 引入流处理
use serde_json::json;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub const SERVERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ssh_servers");
pub const COMMANDS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("quick_commands");
pub const AI_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ai_settings");
pub const SYNC_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sync_config");
pub const REDIS_CONN_TABLE: TableDefinition<&str, &str> = TableDefinition::new("redis_connections");

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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuickCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub current_provider: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
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

    async fn data(&mut self, _channel: ChannelId, data: &[u8], _session: &mut client::Session) -> Result<(), Self::Error> {
        let text = String::from_utf8_lossy(data).to_string();
        let _ = self.window.emit("ssh-output", SshPayload {
            server_id: self.server_id.clone(),
            session_id: self.session_id.clone(),
            data: text,
        });
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
}

pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
    pub db: Arc<redb::Database>,
    pub cancelled_tasks: Arc<Mutex<HashSet<String>>>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(), // 这只是增加引用计数，开销极小
            db: self.db.clone(),
            cancelled_tasks: self.cancelled_tasks.clone(),
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
) -> Result<client::Handle<ClientHandler<R>>, String> {
    let client_config = Arc::new(client::Config::default());
    let handler = ClientHandler {
        window: window.clone(),
        server_id: target_config.id.clone(),
        session_id: session_id.clone()
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
                window.clone(), jump_config, all_configs, format!("{}_tunnel", session_id)
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
        if !server.host.is_empty() {
            server.host = decrypt_secret(&server.host).unwrap_or_else(|_| "DECRYPT_ERROR".into());
        }
        if let Some(ref pass) = server.password {
            server.password = Some(decrypt_secret(pass).unwrap_or_else(|_| "".into()));
        }
        servers.push(server);
    }
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
    println!("  - host: '{}:{}'", server.host, server.port);
    println!("  - username: '{}'", server.username);
    println!("  - auth_type: '{}'", server.auth_type);
    println!("  - jump_host_id: {:?}", server.jump_host_id);

    if server.id.is_empty() {
        server.id = Uuid::new_v4().to_string();
        println!("生成新 ID: {}", server.id);
    }

    if !server.host.is_empty() {
        server.host = encrypt_secret(&server.host)?;
    }
    // 2. 加密 password (如果有)
    if let Some(ref pass) = server.password {
        if !pass.is_empty() {
            server.password = Some(encrypt_secret(pass)?);
        }
    }

    // 确保空的 jump_host_id 被正确处理为 None
    if let Some(ref jump_id) = server.jump_host_id {
        if jump_id.is_empty() {
            server.jump_host_id = None;
            println!("jump_host_id 转换为: None");
        }
    }

    println!("\n处理后的数据:");
    println!("  - id: '{}'", server.id);
    println!("  - jump_host_id (最终): {:?}", server.jump_host_id);

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
            // 触发同步
            trigger_auto_sync(state.inner(), app_handle).await;
            Ok(server)
        }
        Err(e) => {
            println!("✗ 事务提交失败：{:?}", e);
            println!("========== [SAVE_SERVER] 保存失败 ========== {}\n", e);
            Err(e.to_string())
        }
    }
}


#[tauri::command]
async fn delete_server(app_handle: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
        table.remove(id.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    // 触发同步
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
    let servers = get_servers(state.clone()).await?;
    let target_config = servers.iter().find(|s| s.id == server_id)
        .ok_or("配置不存在")?.clone();
    let handle = create_recursive_session(window.clone(), &target_config, &servers, session_id.clone()).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.request_pty(true, "xterm", 80, 24, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
    let channel_id = channel.id();
    state.sessions.lock().await.insert(session_id, ActiveSession { handle, channel_id });
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
    if let Some(_session) = sessions.remove(&session_id) {
        println!("SSH Session {} removed from state.", session_id);
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
        commands.push(cmd);
    }
    Ok(commands)
}

#[tauri::command]
async fn save_quick_command(app_handle: tauri::AppHandle, state: State<'_, AppState>, mut cmd: QuickCommand) -> Result<QuickCommand, String> {
    if cmd.id.is_empty() { cmd.id = Uuid::new_v4().to_string(); }
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

        table.remove(id.as_str()).map_err(|e| e.to_string())?;
    }

    write_txn.commit().map_err(|e| e.to_string())?;
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("无法创建目录");
            }
            let db = Database::builder()
                .create(app_data_dir.join("hiphup_ssh_v1.redb"))
                .expect("无法打开数据库");

            {
                let write_txn = db.begin_write().expect("无法开启写事务");
                {
                    let _ = write_txn.open_table(SERVERS_TABLE).expect("初始化服务器表失败");
                    let _ = write_txn.open_table(COMMANDS_TABLE).expect("初始化命令表失败");
                    let _ = write_txn.open_table(AI_CONFIG_TABLE).expect("初始化AI设置表失败");
                    let _ = write_txn.open_table(SYNC_CONFIG_TABLE).expect("初始化同步设置表失败");
                    let _ = write_txn.open_table(REDIS_CONN_TABLE).expect("初始化Redis连接表失败");
                }
                write_txn.commit().expect("提交初始化事务失败");
            }

            app.manage(AppState {
                sessions: Arc::new(Mutex::new(HashMap::new())),
                db: Arc::new(db),
                cancelled_tasks: Arc::new(Mutex::new(HashSet::new())),
            });

            app.manage(redis_manager::RedisState {
                connection: Arc::new(tokio::sync::Mutex::new(None)),
            });

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
            write_to_ssh,
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
            clear_all_redis_configs
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 运行出错");
}