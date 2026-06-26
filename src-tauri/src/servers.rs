use crate::app_state::{AppState, SERVERS_TABLE};
use crate::security::{decrypt_secret, encrypt_secret};
use crate::ssh_session::measure_server_latency;
use crate::sync::schedule_push_sync;
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

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

pub fn collect_servers_json(db: &Arc<redb::Database>) -> String {
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
            let name_a = a
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let name_b = b
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            name_a.cmp(&name_b)
        });

        serde_json::to_string(&list).ok()
    })()
    .unwrap_or_else(|| "[]".to_string())
}

#[tauri::command]
pub async fn get_servers(state: State<'_, AppState>) -> Result<Vec<ServerConfig>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

    let mut servers = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let mut server: ServerConfig =
            serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

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
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(servers)
}

#[tauri::command]
pub async fn update_server_order(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

        for (index, id) in ids.iter().enumerate() {
            let existing_json = {
                table
                    .get(id.as_str())
                    .map_err(|e| e.to_string())?
                    .map(|v| v.value().to_string())
            };

            if let Some(json_str) = existing_json {
                let mut server: ServerConfig =
                    serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

                server.sort_order = index as i32;
                server.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let new_json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

                table
                    .insert(id.as_str(), new_json.as_str())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;
    schedule_push_sync(state.inner(), app_handle).await;

    Ok(())
}

#[tauri::command]
pub async fn get_server_latency(
    state: State<'_, AppState>,
    server_id: String,
) -> Result<u32, String> {
    let servers = get_servers(state.clone()).await?;
    let target = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or("配置不存在")?;
    measure_server_latency(state.inner(), target, &servers).await
}

#[tauri::command]
pub async fn save_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mut server: ServerConfig,
) -> Result<ServerConfig, String> {
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

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

        println!("\n序列化 JSON: {}", json);
        println!("准备插入数据库...");

        table
            .insert(server.id.as_str(), json.as_str())
            .map_err(|e| e.to_string())?;
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
pub async fn delete_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;

    {
        let mut table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;

        let existing_data = table
            .get(id.as_str())
            .map_err(|e| e.to_string())?
            .map(|v| v.value().to_string());

        if let Some(json_str) = existing_data {
            let mut server: ServerConfig =
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

            server.deleted = true;
            server.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let json = serde_json::to_string(&server).map_err(|e| e.to_string())?;

            table
                .insert(id.as_str(), json.as_str())
                .map_err(|e| e.to_string())?;
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;

    Ok(())
}
