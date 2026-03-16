use crate::security::{
    encrypt_secret, decrypt_secret,
    encrypt_with_key, decrypt_with_key
};
use serde::{Deserialize, Serialize};
use tauri::{State, Emitter};
use crate::{
    AppState, ServerConfig, QuickCommand, AiConfig,
    SERVERS_TABLE, COMMANDS_TABLE, AI_CONFIG_TABLE, SYNC_CONFIG_TABLE, REDIS_CONN_TABLE
};
use redb::ReadableTable;
use crate::redis_manager::RedisConfig;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub struct FullSyncData {
    pub servers: Vec<ServerConfig>,
    pub commands: Vec<QuickCommand>,
    pub ai_config: Option<AiConfig>,
    pub redis_configs: Vec<RedisConfig>,
    pub sync_config: Option<SyncConfig>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub remote_filename: String,
    #[serde(default)]
    pub master_key: String,
    pub auto_sync: bool,
}

// 获取当前毫秒时间戳的辅助函数
fn get_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn get_sync_settings_internal(state: &AppState) -> Result<SyncConfig, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;

    let mut config = if let Some(v) = table.get("default").map_err(|e| e.to_string())? {
        serde_json::from_str::<SyncConfig>(v.value()).map_err(|e| e.to_string())?
    } else {
        SyncConfig {
            endpoint: "".into(),
            username: "".into(),
            password: "".into(),
            remote_filename: "ssh_sync_backup.enc".into(),
            master_key: "".into(),
            auto_sync: false,
        }
    };

    if !config.password.is_empty() {
        config.password = decrypt_secret(&config.password).unwrap_or_default();
    }
    if !config.master_key.is_empty() {
        config.master_key = decrypt_secret(&config.master_key).unwrap_or_default();
    }
    Ok(config)
}

pub async fn sync_to_cloud_internal(state: &AppState, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() {
        return Err("主加密密钥（Master Key）不能为空".into());
    }

    let data_json = state.export_all_data().await?;
    let encrypted_bytes = encrypt_with_key(&data_json, &config.master_key)?;

    let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;

    let url = format!("{}/{}", config.endpoint.trim_end_matches('/'), config.remote_filename);

    let res = client.put(url)
        .basic_auth(&config.username, Some(&config.password))
        .body(encrypted_bytes)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if res.status().is_success() {
        Ok("加密同步成功".into())
    } else {
        Err(format!("服务器返回错误: {}", res.status()))
    }
}

impl AppState {
    pub async fn export_all_data(&self) -> Result<String, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;

        let mut servers = Vec::new();
        if let Ok(table) = read_txn.open_table(SERVERS_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(s) = serde_json::from_str(v.value()) { servers.push(s); }
            }
        }

        let mut commands = Vec::new();
        if let Ok(table) = read_txn.open_table(COMMANDS_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(c) = serde_json::from_str(v.value()) { commands.push(c); }
            }
        }

        let mut ai_config = None;
        if let Ok(table) = read_txn.open_table(AI_CONFIG_TABLE) {
            if let Ok(Some(v)) = table.get("default") {
                ai_config = serde_json::from_str(v.value()).ok();
            }
        }

        let mut redis_configs = Vec::new();
        if let Ok(table) = read_txn.open_table(REDIS_CONN_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(r) = serde_json::from_str::<RedisConfig>(v.value()) {
                    redis_configs.push(r);
                }
            }
        }

        let mut sync_config = None;
        if let Ok(table) = read_txn.open_table(SYNC_CONFIG_TABLE) {
            if let Ok(Some(v)) = table.get("default") {
                if let Ok(mut sc) = serde_json::from_str::<SyncConfig>(v.value()) {
                    sc.master_key = String::new(); // 安全策略：不备份加密主密钥
                    sync_config = Some(sc);
                }
            }
        }

        let sync_data = FullSyncData {
            servers,
            commands,
            ai_config,
            redis_configs,
            sync_config,
            timestamp: get_now_ms(),
        };

        serde_json::to_string(&sync_data).map_err(|e| e.to_string())
    }

    pub async fn import_all_data(&self, json_str: String) -> Result<bool, String> {
        let data: FullSyncData = serde_json::from_str(&json_str)
            .map_err(|_| "备份文件格式无效或密钥错误".to_string())?;

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        let mut changed = false;

        {
            // 抽象通用合并逻辑 (LWW)
            let mut merge_entities = |table_def: redb::TableDefinition<&str, &str>, incoming_vals: Vec<serde_json::Value>| -> Result<(), String> {
                let mut table = write_txn.open_table(table_def).map_err(|e| e.to_string())?;
                for val in incoming_vals {
                    let id = val["id"].as_str().ok_or("Missing ID")?;
                    let incoming_ts = val["updated_at"].as_u64().unwrap_or(0);

                    let should_update = if let Some(local_raw) = table.get(id).map_err(|e| e.to_string())? {
                        let local_val: serde_json::Value = serde_json::from_str(local_raw.value()).unwrap();
                        let local_ts = local_val["updated_at"].as_u64().unwrap_or(0);
                        incoming_ts > local_ts
                    } else {
                        true
                    };

                    if should_update {
                        table.insert(id, serde_json::to_string(&val).unwrap().as_str()).ok();
                        changed = true;
                    }
                }
                Ok(())
            };

            // 1. 合并服务器
            let servers_v = data.servers.into_iter().map(|s| serde_json::to_value(s).unwrap()).collect();
            merge_entities(SERVERS_TABLE, servers_v)?;

            // 2. 合并命令
            let commands_v = data.commands.into_iter().map(|c| serde_json::to_value(c).unwrap()).collect();
            merge_entities(COMMANDS_TABLE, commands_v)?;

            // 3. 合并 Redis
            let redis_v = data.redis_configs.into_iter().filter(|r| r.id.is_some())
                .map(|r| serde_json::to_value(r).unwrap()).collect();
            merge_entities(REDIS_CONN_TABLE, redis_v)?;

            // 4. 合并 AI 配置 (单记录处理)
            if let Some(ai) = data.ai_config {
                let mut ai_table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                let incoming_ts = ai.updated_at;
                let should_ai = if let Some(l) = ai_table.get("default").map_err(|e| e.to_string())? {
                    let local_ai: AiConfig = serde_json::from_str(l.value()).unwrap();
                    incoming_ts > local_ai.updated_at
                } else { true };

                if should_ai {
                    ai_table.insert("default", serde_json::to_string(&ai).unwrap().as_str()).ok();
                    changed = true;
                }
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(changed)
    }
}

pub async fn sync_from_cloud_internal(state: &AppState, config: &SyncConfig) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build().map_err(|e| e.to_string())?;

    let url = format!("{}/{}", config.endpoint.trim_end_matches('/'), config.remote_filename);
    let res = client.get(url)
        .basic_auth(&config.username, Some(&config.password))
        .send().await.map_err(|e| e.to_string())?;

    if res.status().is_success() {
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        let json = decrypt_with_key(&bytes, &config.master_key)?;
        // 返回是否有数据变更
        state.import_all_data(json).await
    } else {
        Err(format!("状态码: {}", res.status()))
    }
}

pub async fn trigger_auto_sync(state: &AppState, app_handle: tauri::AppHandle) {
    let state_handle = state.clone();
    tauri::async_runtime::spawn(async move {
        let config = match get_sync_settings_internal(&state_handle).await {
            Ok(c) if c.auto_sync && !c.endpoint.is_empty() && !c.master_key.is_empty() => c,
            _ => return,
        };

        let _ = app_handle.emit("sync-status", true);

        // 先拉取合并
        match sync_from_cloud_internal(&state_handle, &config).await {
            Ok(changed) => {
                if changed {
                    let _ = app_handle.emit("database-changed", "auto-sync");
                }
            },
            Err(e) => println!("[AutoSync] 拉取跳过: {}", e),
        }

        // 后推送更新
        if let Err(e) = sync_to_cloud_internal(&state_handle, config).await {
            let _ = app_handle.emit("sync-error", format!("上传失败: {}", e));
        } else {
            let _ = app_handle.emit("sync-finished", "同步成功");
        }

        let _ = app_handle.emit("sync-status", false);
    });
}

#[tauri::command]
pub async fn save_sync_settings(app_handle: tauri::AppHandle, state: State<'_, AppState>, mut config: SyncConfig) -> Result<(), String> {
    if !config.password.is_empty() { config.password = encrypt_secret(&config.password)?; }
    if !config.master_key.is_empty() { config.master_key = encrypt_secret(&config.master_key)?; }

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
        table.insert("default", serde_json::to_string(&config).unwrap().as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;

    trigger_auto_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn sync_from_cloud(app_handle: tauri::AppHandle, state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() { return Err("Master Key 缺失".into()); }
    sync_from_cloud_internal(state.inner(), &config).await?;
    let _ = app_handle.emit("database-changed", "manual-sync");
    Ok("同步成功".into())
}

#[tauri::command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    get_sync_settings_internal(state.inner()).await
}

#[tauri::command]
pub async fn sync_to_cloud(state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    sync_to_cloud_internal(state.inner(), config).await
}