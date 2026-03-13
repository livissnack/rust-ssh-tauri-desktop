use crate::security::{
    encrypt_secret, decrypt_secret,
    encrypt_with_key, decrypt_with_key
};
use serde::{Deserialize, Serialize};
use tauri::{State, Emitter};
use crate::{
    AppState, ServerConfig, QuickCommand, AiConfig,
    SERVERS_TABLE, COMMANDS_TABLE, AI_CONFIG_TABLE, SYNC_CONFIG_TABLE
};
use redb::ReadableTable;
use crate::redis_manager::RedisConfig;
use std::time::Duration;

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
            .timeout(Duration::from_secs(5)) // 设置 5 秒超时
            .connect_timeout(Duration::from_secs(3)) // 设置 3 秒连接超时
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
        if let Ok(table) = read_txn.open_table(crate::REDIS_CONN_TABLE) {
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
                    sc.master_key = String::new(); // 敏感数据不进入备份明文
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
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        serde_json::to_string(&sync_data).map_err(|e| e.to_string())
    }

    pub async fn import_all_data(&self, json_str: String) -> Result<(), String> {
        let data: FullSyncData = serde_json::from_str(&json_str)
            .map_err(|_| "备份文件格式无效或密钥错误".to_string())?;

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut s_table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
            for s in data.servers {
                s_table.insert(s.id.as_str(), serde_json::to_string(&s).unwrap().as_str()).ok();
            }

            let mut c_table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
            for c in data.commands {
                c_table.insert(c.id.as_str(), serde_json::to_string(&c).unwrap().as_str()).ok();
            }

            let mut r_table = write_txn.open_table(crate::REDIS_CONN_TABLE).map_err(|e| e.to_string())?;
            for r in data.redis_configs {
                if let Some(ref id_str) = r.id {
                    let val = serde_json::to_string(&r).unwrap();
                    r_table.insert(id_str.as_str(), val.as_str()).ok();
                }
            }

            if let Some(ai) = data.ai_config {
                let mut ai_table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                ai_table.insert("default", serde_json::to_string(&ai).unwrap().as_str()).ok();
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub async fn trigger_auto_sync(state: &AppState, app_handle: tauri::AppHandle) {
    let state_handle = state.clone();

    tauri::async_runtime::spawn(async move {
        // 获取配置
        let config = match get_sync_settings_internal(&state_handle).await {
            Ok(c) => c,
            Err(_) => return, // 配置读取失败直接退出
        };

        // 如果没开启自动同步或配置不全，直接静默退出
        if !config.auto_sync || config.endpoint.is_empty() || config.master_key.is_empty() {
            return;
        }

        // 开始同步
        let _ = app_handle.emit("sync-status", true);

        // 真实的执行同步
        match sync_to_cloud_internal(&state_handle, config).await {
            Ok(_) => {
                println!("[AutoSync] 自动同步成功");
                // 可以选发一个成功消息
                let _ = app_handle.emit("sync-finished", "同步成功");
            },
            Err(e) => {
                eprintln!("[AutoSync] 自动同步失败: {}", e);
                // 🚀 关键：将错误发送给前端
                let _ = app_handle.emit("sync-error", format!("自动同步失败: {}", e));
            },
        }

        // 结束状态
        let _ = app_handle.emit("sync-status", false);
    });
}

#[tauri::command]
pub async fn save_sync_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    mut config: SyncConfig
) -> Result<(), String> {
    if !config.password.is_empty() {
        config.password = encrypt_secret(&config.password)?;
    }
    if !config.master_key.is_empty() {
        config.master_key = encrypt_secret(&config.master_key)?;
    }

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
        let val = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;

    trigger_auto_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    get_sync_settings_internal(state.inner()).await
}

#[tauri::command]
pub async fn sync_to_cloud(state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    sync_to_cloud_internal(state.inner(), config).await
}

#[tauri::command]
pub async fn sync_from_cloud(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: SyncConfig
) -> Result<String, String> {
    if config.master_key.is_empty() {
        return Err("必须输入主加密密钥以进行解密".into());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/{}", config.endpoint.trim_end_matches('/'), config.remote_filename);

    let res = client.get(url)
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;

    if res.status().is_success() {
        let encrypted_bytes = res.bytes().await.map_err(|e| e.to_string())?;

        let json_str = decrypt_with_key(&encrypted_bytes, &config.master_key)?;

        state.import_all_data(json_str).await?;

        let _ = app_handle.emit("database-changed", "sync");
        Ok("解密并同步成功".into())
    } else {
        Err(format!("下载失败: 状态码 {}", res.status()))
    }
}