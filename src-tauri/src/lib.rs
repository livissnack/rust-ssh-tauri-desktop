mod app_state;
mod quick_commands;
mod servers;
mod ssh_session;
mod sftp;
mod session_window;
mod sync;
mod p2p;
mod security;
mod redis_manager;
mod local_shell;
mod api_debugger;
mod ai_chat;
mod port_forward;
mod known_hosts;

pub use app_state::{
    AppState, SyncRuntime, AI_CHAT_SESSIONS_TABLE, AI_CONFIG_TABLE, API_DEBUGGER_TABLE,
    COMMANDS_TABLE, P2P_MESSAGES_TABLE, P2P_REMARKS_TABLE, REDIS_CONN_TABLE, SERVERS_TABLE,
    SYNC_CONFIG_TABLE,
};
pub use quick_commands::QuickCommand;
pub use servers::ServerConfig;
pub use ssh_session::ClientHandler;

use crate::sync::{
    get_sync_settings, save_sync_settings, sync_to_cloud, sync_from_cloud, schedule_push_sync,
    run_startup_pull,
};
use security::{encrypt_secret, decrypt_secret};
use p2p::{set_p2p_remark, start_p2p_node, get_p2p_remarks, search_p2p_messages, get_online_peers};
use api_debugger::{
    export_api_debugger_file, get_api_debugger_data, import_api_debugger_file, save_api_debugger_data,
};
use ai_chat::{
    clear_ai_chat_sessions, delete_ai_chat_session, get_ai_chat_session, list_ai_chat_sessions,
    prune_ai_chat_sessions, save_ai_chat_session,
};
use known_hosts::{
    list_known_hosts, remove_known_host, respond_host_key_prompt, HostKeyPromptHub, KNOWN_HOSTS_TABLE,
};
use port_forward::{list_port_forwards, start_port_forward, stop_port_forward, new_port_forward_map};
use redis_manager::{
    redis_connect, redis_get_keys, redis_get_value, redis_set_value, redis_del_key, redis_rename_key,
    redis_get_ttl, redis_get_type, save_redis_config, get_redis_configs, delete_redis_config,
    clear_all_redis_configs,
};
use quick_commands::{delete_quick_command, get_quick_commands, save_quick_command};
use servers::{
    delete_server, get_server_latency, get_servers, save_server, update_server_order,
};
use session_window::{open_session_window, preheat_servers};
use sftp::{
    abort_transfer, copy_local_path, copy_remote_path, create_local_path, create_remote_path,
    delete_local_file, delete_remote_file, get_local_file_info, get_remote_file_info, list_local_dir,
    list_remote_dir, move_local_path, move_remote_path, pause_transfer, rename_local_file,
    rename_remote_file, resume_transfer, reveal_in_file_manager, set_remote_file_permissions,
    sftp_download, sftp_upload,
};
use ssh_session::{connect_ssh, disconnect_ssh, resize_ssh, write_to_ssh};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use futures::StreamExt;
use redb::{Database, ReadableTable};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State, Window};
use tokio::sync::Mutex;

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
async fn write_text_file(path: String, content: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || std::fs::write(&path, content))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_ai_config(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    mut config: AiConfig,
) -> Result<(), String> {
    if !config.api_key.is_empty() {
        config.api_key = encrypt_secret(&config.api_key)?;
    }
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn
            .open_table(AI_CONFIG_TABLE)
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table
            .insert("default", json.as_str())
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    schedule_push_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
async fn get_ai_config(state: State<'_, AppState>) -> Result<Option<AiConfig>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn
        .open_table(AI_CONFIG_TABLE)
        .map_err(|e| e.to_string())?;

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
            let _ = window.emit(
                "ai-res-chunk",
                json!({ "taskId": &task_id, "content": format!("❌ 网络请求失败: {}", e) }),
            );
            e.to_string()
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let err_text = response.text().await.unwrap_or_else(|_| "无法读取错误详情".into());
        let detailed_err = if let Ok(json_err) = serde_json::from_str::<serde_json::Value>(&err_text) {
            json_err["error"]["message"]
                .as_str()
                .unwrap_or(&err_text)
                .to_string()
        } else {
            err_text
        };
        let final_err = format!("API 错误 ({}): {}", status, detailed_err);
        let _ = window.emit(
            "ai-res-chunk",
            json!({ "taskId": &task_id, "content": format!("❌ {}", final_err) }),
        );
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

                    if line.is_empty() {
                        continue;
                    }
                    if line == "data: [DONE]" {
                        return Ok(());
                    }

                    if let Some(data_json) = line.strip_prefix("data: ") {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data_json) {
                            if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                                window
                                    .emit(
                                        "ai-res-chunk",
                                        json!({
                                            "taskId": &task_id,
                                            "content": content
                                        }),
                                    )
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = window.emit(
                    "ai-res-chunk",
                    json!({ "taskId": &task_id, "content": format!("\n[流传输中断: {}]", e) }),
                );
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
    println!(
        "[Command] 准备发送消息到 P2P 队列: target={}, content={}",
        target, content
    );

    state
        .p2p_sender
        .send(p2p::P2PCommand::SendMessage { target, content })
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
    if !path_buf.exists() {
        return Err("文件不存在".into());
    }

    state
        .p2p_sender
        .send(p2p::P2PCommand::SendFile {
            target,
            path: path_buf,
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_p2p_messages(
    state: tauri::State<'_, AppState>,
    peer_id: String,
) -> Result<Vec<p2p::ChatMessageRecord>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn
        .open_table(P2P_MESSAGES_TABLE)
        .map_err(|e| e.to_string())?;

    let mut msgs = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let msg: p2p::ChatMessageRecord =
            serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if msg.peer_id == peer_id {
            msgs.push(msg);
        }
    }

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

            let app_data_dir = handle.path().app_data_dir().expect("无法获取应用数据目录");
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("无法创建目录");
            }
            let db_path = app_data_dir.join("hiphup_ssh_v1.redb");

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

            app.manage(AppState {
                sessions: Arc::new(Mutex::new(HashMap::new())),
                local_sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
                db: db_arc.clone(),
                cancelled_tasks: Arc::new(Mutex::new(HashSet::new())),
                paused_tasks: Arc::new(Mutex::new(HashSet::new())),
                p2p_sender: tx,
                sync_runtime: Arc::new(tokio::sync::Mutex::new(SyncRuntime::default())),
                port_forwards: new_port_forward_map(),
                app_handle: handle.clone(),
                host_key_hub: Arc::new(HostKeyPromptHub::new()),
            });

            app.manage(shared_p2p_status.clone());

            let handle_for_startup = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                if let Some(state) = handle_for_startup.try_state::<AppState>() {
                    run_startup_pull(state.inner(), handle_for_startup.clone()).await;
                }
            });

            let handle_for_p2p = app.handle().clone();
            let status_for_node = shared_p2p_status.clone();
            let db_for_p2p = db_arc.clone();

            tauri::async_runtime::spawn(async move {
                let _ = start_p2p_node(handle_for_p2p, rx, db_for_p2p, status_for_node).await;
            });

            app.manage(redis_manager::RedisState {
                connection: Arc::new(tokio::sync::Mutex::new(None)),
            });

            if let Some(main_window) = app.get_webview_window("main") {
                let win = main_window.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_focus();
                    let _ = win.set_always_on_top(false);
                });
            }

            let db_for_setup = db_arc.clone();

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let init_res = (|| -> Result<(), String> {
                    let write_txn = db_for_setup.begin_write().map_err(|e| e.to_string())?;
                    {
                        let _ = write_txn
                            .open_table(SERVERS_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(COMMANDS_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(AI_CONFIG_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(SYNC_CONFIG_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(REDIS_CONN_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(P2P_MESSAGES_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(P2P_REMARKS_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(API_DEBUGGER_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(AI_CHAT_SESSIONS_TABLE)
                            .map_err(|e| e.to_string())?;
                        let _ = write_txn
                            .open_table(KNOWN_HOSTS_TABLE)
                            .map_err(|e| e.to_string())?;
                    }
                    write_txn.commit().map_err(|e| e.to_string())?;
                    let _ = ai_chat::prune_expired_sessions(&db_for_setup);
                    Ok(())
                })();

                if let Err(e) = init_res {
                    eprintln!("[DB Error] 初始化失败: {}", e);
                }
            });

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
            write_text_file,
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
            pause_transfer,
            resume_transfer,
            delete_remote_file,
            get_quick_commands,
            save_quick_command,
            delete_quick_command,
            save_ai_config,
            get_ai_config,
            ask_ai,
            list_ai_chat_sessions,
            get_ai_chat_session,
            save_ai_chat_session,
            delete_ai_chat_session,
            clear_ai_chat_sessions,
            prune_ai_chat_sessions,
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
            import_api_debugger_file,
            start_port_forward,
            stop_port_forward,
            list_port_forwards,
            respond_host_key_prompt,
            list_known_hosts,
            remove_known_host,
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
            } = event
            {
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
