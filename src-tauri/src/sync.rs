use crate::redis_manager::RedisConfig;
use crate::security::{decrypt_secret, encrypt_secret, encrypt_with_key, decrypt_with_key};
use crate::{
    AiConfig, AppState, QuickCommand, ServerConfig, AI_CONFIG_TABLE, COMMANDS_TABLE,
    REDIS_CONN_TABLE, SERVERS_TABLE, SYNC_CONFIG_TABLE,
};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, State};
use uuid::Uuid;

const SYNC_META_TABLE_KEY: &str = "sync_meta";
const PUSH_DEBOUNCE: Duration = Duration::from_millis(2000);

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LocalSyncMeta {
    pub device_id: String,
    pub local_revision: u64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RemoteSyncMeta {
    pub revision: u64,
    pub updated_at: u64,
    pub device_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct FullSyncData {
    pub servers: Vec<ServerConfig>,
    pub commands: Vec<QuickCommand>,
    pub ai_config: Option<AiConfig>,
    pub redis_configs: Vec<RedisConfig>,
    pub sync_config: Option<SyncConfig>,
    pub timestamp: u64,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub device_id: String,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncErrorPayload {
    pub phase: String,
    pub message: String,
}

fn get_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn try_decrypt_field(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    decrypt_secret(value).unwrap_or_else(|_| value.to_string())
}

fn meta_filename(remote_filename: &str) -> String {
    format!("{}.meta.json", remote_filename)
}

fn build_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())
}

fn data_url(config: &SyncConfig) -> String {
    format!(
        "{}/{}",
        config.endpoint.trim_end_matches('/'),
        config.remote_filename
    )
}

fn meta_url(config: &SyncConfig) -> String {
    format!(
        "{}/{}",
        config.endpoint.trim_end_matches('/'),
        meta_filename(&config.remote_filename)
    )
}

fn read_local_sync_meta(db: &redb::Database) -> Result<LocalSyncMeta, String> {
    let read_txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
    if let Some(v) = table.get(SYNC_META_TABLE_KEY).map_err(|e| e.to_string())? {
        return serde_json::from_str(v.value()).map_err(|e| e.to_string());
    }
    Ok(LocalSyncMeta::default())
}

fn write_local_sync_meta(db: &redb::Database, meta: &LocalSyncMeta) -> Result<(), String> {
    let write_txn = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
        table
            .insert(
                SYNC_META_TABLE_KEY,
                serde_json::to_string(meta).map_err(|e| e.to_string())?.as_str(),
            )
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())
}

fn ensure_device_id(meta: &mut LocalSyncMeta) -> String {
    if meta.device_id.is_empty() {
        meta.device_id = Uuid::new_v4().to_string();
    }
    meta.device_id.clone()
}

fn decrypt_server_for_sync(mut server: ServerConfig) -> ServerConfig {
    if !server.host.is_empty() {
        server.host = try_decrypt_field(&server.host);
    }
    if let Some(ref pass) = server.password {
        if !pass.is_empty() {
            server.password = Some(try_decrypt_field(pass));
        }
    }
    server
}

fn encrypt_server_for_storage(mut server: ServerConfig) -> Result<ServerConfig, String> {
    if !server.host.is_empty() && server.host != "DECRYPT_ERROR" {
        server.host = encrypt_secret(&server.host)?;
    }
    if let Some(ref pass) = server.password {
        if !pass.is_empty() {
            server.password = Some(encrypt_secret(pass)?);
        }
    }
    Ok(server)
}

fn decrypt_ai_for_sync(mut config: AiConfig) -> AiConfig {
    if !config.api_key.is_empty() {
        config.api_key = try_decrypt_field(&config.api_key);
    }
    config
}

fn encrypt_ai_for_storage(mut config: AiConfig) -> Result<AiConfig, String> {
    if !config.api_key.is_empty() {
        config.api_key = encrypt_secret(&config.api_key)?;
    }
    Ok(config)
}

fn decrypt_redis_for_sync(mut config: RedisConfig) -> RedisConfig {
    if !config.host.is_empty() {
        config.host = try_decrypt_field(&config.host);
    }
    if let Some(ref pass) = config.password {
        if !pass.is_empty() {
            config.password = Some(try_decrypt_field(pass));
        }
    }
    config
}

fn encrypt_redis_for_storage(mut config: RedisConfig) -> Result<RedisConfig, String> {
    if !config.host.is_empty() && config.host != "DECRYPT_ERROR" {
        config.host = encrypt_secret(&config.host)?;
    }
    if let Some(ref pass) = config.password {
        if !pass.is_empty() {
            config.password = Some(encrypt_secret(pass)?);
        }
    }
    Ok(config)
}

fn decrypt_sync_config_for_export(mut config: SyncConfig) -> SyncConfig {
    config.master_key = String::new();
    if !config.password.is_empty() {
        config.password = try_decrypt_field(&config.password);
    }
    config
}

async fn fetch_remote_meta(config: &SyncConfig) -> Result<Option<RemoteSyncMeta>, String> {
    let client = build_http_client()?;
    let res = client
        .get(meta_url(config))
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
        .map_err(|e| format!("读取云端版本失败: {}", e))?;

    if res.status().is_success() {
        let text = res.text().await.map_err(|e| e.to_string())?;
        Ok(Some(
            serde_json::from_str(&text).map_err(|e| format!("云端版本文件无效: {}", e))?,
        ))
    } else if res.status() == reqwest::StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        Err(format!("读取云端版本失败: {}", res.status()))
    }
}

async fn upload_remote_meta(config: &SyncConfig, meta: &RemoteSyncMeta) -> Result<(), String> {
    let client = build_http_client()?;
    let body = serde_json::to_string(meta).map_err(|e| e.to_string())?;
    let res = client
        .put(meta_url(config))
        .basic_auth(&config.username, Some(&config.password))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("上传云端版本失败: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("上传云端版本失败: {}", res.status()))
    }
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

impl AppState {
    pub async fn export_all_data(&self, next_revision: u64, device_id: &str) -> Result<String, String> {
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;

        let mut servers = Vec::new();
        if let Ok(table) = read_txn.open_table(SERVERS_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(s) = serde_json::from_str::<ServerConfig>(v.value()) {
                    servers.push(decrypt_server_for_sync(s));
                }
            }
        }

        let mut commands = Vec::new();
        if let Ok(table) = read_txn.open_table(COMMANDS_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(c) = serde_json::from_str::<QuickCommand>(v.value()) {
                    commands.push(c);
                }
            }
        }

        let mut ai_config = None;
        if let Ok(table) = read_txn.open_table(AI_CONFIG_TABLE) {
            if let Ok(Some(v)) = table.get("default") {
                if let Ok(ai) = serde_json::from_str::<AiConfig>(v.value()) {
                    ai_config = Some(decrypt_ai_for_sync(ai));
                }
            }
        }

        let mut redis_configs = Vec::new();
        if let Ok(table) = read_txn.open_table(REDIS_CONN_TABLE) {
            for res in table.iter().map_err(|e| e.to_string())? {
                let (_, v) = res.map_err(|e| e.to_string())?;
                if let Ok(r) = serde_json::from_str::<RedisConfig>(v.value()) {
                    redis_configs.push(decrypt_redis_for_sync(r));
                }
            }
        }

        let mut sync_config = None;
        if let Ok(table) = read_txn.open_table(SYNC_CONFIG_TABLE) {
            if let Ok(Some(v)) = table.get("default") {
                if let Ok(sc) = serde_json::from_str::<SyncConfig>(v.value()) {
                    sync_config = Some(decrypt_sync_config_for_export(sc));
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
            revision: next_revision,
            device_id: device_id.to_string(),
        };

        serde_json::to_string(&sync_data).map_err(|e| e.to_string())
    }

    pub async fn import_all_data(&self, json_str: String) -> Result<bool, String> {
        let data: FullSyncData = serde_json::from_str(&json_str)
            .map_err(|_| "备份文件格式无效或密钥错误".to_string())?;

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        let mut changed = false;

        {
            let mut merge_entities = |table_def: redb::TableDefinition<&str, &str>,
                                      incoming_vals: Vec<serde_json::Value>|
             -> Result<(), String> {
                let mut table = write_txn.open_table(table_def).map_err(|e| e.to_string())?;
                for val in incoming_vals {
                    let id = val["id"].as_str().ok_or("Missing ID")?;
                    let incoming_ts = val["updated_at"].as_u64().unwrap_or(0);

                    let should_update =
                        if let Some(local_raw) = table.get(id).map_err(|e| e.to_string())? {
                            let local_val: serde_json::Value =
                                serde_json::from_str(local_raw.value()).unwrap();
                            let local_ts = local_val["updated_at"].as_u64().unwrap_or(0);
                            incoming_ts > local_ts
                        } else {
                            true
                        };

                    if should_update {
                        table
                            .insert(id, serde_json::to_string(&val).unwrap().as_str())
                            .ok();
                        changed = true;
                    }
                }
                Ok(())
            };

            let servers_v: Vec<serde_json::Value> = data
                .servers
                .into_iter()
                .map(|s| {
                    encrypt_server_for_storage(s).and_then(|s| {
                        serde_json::to_value(s).map_err(|e| e.to_string())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            merge_entities(SERVERS_TABLE, servers_v)?;

            let commands_v: Vec<serde_json::Value> = data
                .commands
                .into_iter()
                .map(|c| serde_json::to_value(c).unwrap())
                .collect();
            merge_entities(COMMANDS_TABLE, commands_v)?;

            let redis_v: Vec<serde_json::Value> = data
                .redis_configs
                .into_iter()
                .filter(|r| r.id.is_some())
                .map(|r| {
                    encrypt_redis_for_storage(r).and_then(|r| {
                        serde_json::to_value(r).map_err(|e| e.to_string())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            merge_entities(REDIS_CONN_TABLE, redis_v)?;

            if let Some(ai) = data.ai_config {
                let mut ai_table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                let incoming_ts = ai.updated_at;
                let should_ai = if let Some(l) = ai_table.get("default").map_err(|e| e.to_string())? {
                    let local_ai: AiConfig = serde_json::from_str(l.value()).unwrap();
                    incoming_ts > local_ai.updated_at
                } else {
                    true
                };

                if should_ai {
                    let ai = encrypt_ai_for_storage(ai)?;
                    ai_table
                        .insert("default", serde_json::to_string(&ai).unwrap().as_str())
                        .ok();
                    changed = true;
                }
            }
        }

        write_txn.commit().map_err(|e| e.to_string())?;

        if data.revision > 0 {
            let mut meta = read_local_sync_meta(&self.db).unwrap_or_default();
            if data.revision > meta.local_revision {
                meta.local_revision = data.revision;
            }
            ensure_device_id(&mut meta);
            let _ = write_local_sync_meta(&self.db, &meta);
        }

        Ok(changed)
    }
}

pub async fn sync_from_cloud_internal(state: &AppState, config: &SyncConfig) -> Result<bool, String> {
    let client = build_http_client()?;
    let res = client
        .get(data_url(config))
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;

    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }

    if !res.status().is_success() {
        return Err(format!("下载失败: {}", res.status()));
    }

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    let json = decrypt_with_key(&bytes, &config.master_key)?;
    state.import_all_data(json).await
}

async fn sync_to_cloud_internal(state: &AppState, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() {
        return Err("主加密密钥（Master Key）不能为空".into());
    }

    let mut local_meta = read_local_sync_meta(&state.db).unwrap_or_default();
    let device_id = ensure_device_id(&mut local_meta);
    let _ = write_local_sync_meta(&state.db, &local_meta);

    if let Some(remote_meta) = fetch_remote_meta(&config).await? {
        if remote_meta.revision > local_meta.local_revision {
            let _ = sync_from_cloud_internal(state, &config).await?;
            local_meta = read_local_sync_meta(&state.db).unwrap_or_default();
        }
    }

    let next_revision = local_meta.local_revision.saturating_add(1);
    let data_json = state.export_all_data(next_revision, &device_id).await?;
    let encrypted_bytes = encrypt_with_key(&data_json, &config.master_key)?;

    let client = build_http_client()?;
    let res = client
        .put(data_url(&config))
        .basic_auth(&config.username, Some(&config.password))
        .body(encrypted_bytes)
        .send()
        .await
        .map_err(|e| format!("上传失败: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("上传失败: {}", res.status()));
    }

    let remote_meta = RemoteSyncMeta {
        revision: next_revision,
        updated_at: get_now_ms(),
        device_id: device_id.clone(),
    };
    upload_remote_meta(&config, &remote_meta).await?;

    local_meta.local_revision = next_revision;
    local_meta.device_id = device_id;
    write_local_sync_meta(&state.db, &local_meta)?;

    Ok(format!("已同步到云端 (revision {})", next_revision))
}

fn emit_sync_error(app: &tauri::AppHandle, phase: &str, message: impl Into<String>) {
    let payload = SyncErrorPayload {
        phase: phase.to_string(),
        message: message.into(),
    };
    let _ = app.emit("sync-error", payload);
}

async fn execute_push_sync(state: &AppState, app: &tauri::AppHandle) -> Result<(), String> {
    let config = get_sync_settings_internal(state).await?;
    if !config.auto_sync {
        return Ok(());
    }
    if config.endpoint.is_empty() || config.master_key.is_empty() {
        return Ok(());
    }

    let _ = app.emit("sync-status", true);
    let result = sync_to_cloud_internal(state, config).await;
    let _ = app.emit("sync-status", false);

    match result {
        Ok(msg) => {
            let _ = app.emit("sync-finished", msg);
            Ok(())
        }
        Err(e) => {
            emit_sync_error(app, "upload", e.clone());
            Err(e)
        }
    }
}

async fn execute_pull_sync(state: &AppState, app: &tauri::AppHandle) -> Result<bool, String> {
    let config = get_sync_settings_internal(state).await?;
    if config.endpoint.is_empty() || config.master_key.is_empty() {
        return Ok(false);
    }

    let _ = app.emit("sync-status", true);
    let result = sync_from_cloud_internal(state, &config).await;
    let _ = app.emit("sync-status", false);

    match result {
        Ok(changed) => {
            if changed {
                let _ = app.emit("database-changed", "startup-pull");
            }
            Ok(changed)
        }
        Err(e) => {
            emit_sync_error(app, "download", e.clone());
            Err(e)
        }
    }
}

async fn debounce_push_worker(state: AppState, app: tauri::AppHandle) {
    tokio::time::sleep(PUSH_DEBOUNCE).await;

    loop {
        {
            let mut rt = state.sync_runtime.lock().await;
            rt.debounce_active = false;
            if !rt.pending_push {
                return;
            }
            if rt.sync_in_progress {
                rt.debounce_active = true;
                drop(rt);
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            rt.pending_push = false;
            rt.sync_in_progress = true;
        }

        let _ = execute_push_sync(&state, &app).await;

        let reschedule = {
            let mut rt = state.sync_runtime.lock().await;
            rt.sync_in_progress = false;
            if rt.pending_push {
                rt.debounce_active = true;
                true
            } else {
                false
            }
        };

        if reschedule {
            tokio::time::sleep(PUSH_DEBOUNCE).await;
            continue;
        }
        return;
    }
}

pub async fn schedule_push_sync(state: &AppState, app_handle: tauri::AppHandle) {
    let should_spawn = {
        let mut rt = state.sync_runtime.lock().await;
        rt.pending_push = true;
        if !rt.debounce_active && !rt.sync_in_progress {
            rt.debounce_active = true;
            true
        } else {
            false
        }
    };

    if should_spawn {
        let state_handle = state.clone();
        tauri::async_runtime::spawn(async move {
            debounce_push_worker(state_handle, app_handle).await;
        });
    }
}

pub async fn run_startup_pull(state: &AppState, app_handle: tauri::AppHandle) {
    let config = match get_sync_settings_internal(state).await {
        Ok(c) if c.auto_sync && !c.endpoint.is_empty() && !c.master_key.is_empty() => c,
        _ => return,
    };

    let state_handle = state.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        if let Ok(remote_meta) = fetch_remote_meta(&config).await {
            let local_meta = read_local_sync_meta(&state_handle.db).unwrap_or_default();
            if remote_meta
                .map(|m| m.revision > local_meta.local_revision)
                .unwrap_or(true)
            {
                let _ = execute_pull_sync(&state_handle, &app_handle).await;
            }
        }
    });
}

#[tauri::command]
pub async fn save_sync_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    mut config: SyncConfig,
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
        table
            .insert("default", serde_json::to_string(&config).unwrap().as_str())
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn sync_from_cloud(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    config: SyncConfig,
) -> Result<String, String> {
    if config.master_key.is_empty() {
        return Err("Master Key 缺失".into());
    }
    let changed = sync_from_cloud_internal(state.inner(), &config).await?;
    if changed {
        let _ = app_handle.emit("database-changed", "manual-sync");
    }
    Ok(if changed {
        "已从云端合并最新配置".into()
    } else {
        "云端暂无备份或无需更新".into()
    })
}

#[tauri::command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    get_sync_settings_internal(state.inner()).await
}

#[tauri::command]
pub async fn sync_to_cloud(state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    sync_to_cloud_internal(state.inner(), config).await
}
