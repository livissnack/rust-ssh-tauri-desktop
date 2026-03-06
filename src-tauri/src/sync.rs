use serde::{Deserialize, Serialize};
use tauri::{State, command};
use crate::{
    AppState, ServerConfig, QuickCommand, AiConfig,
    SERVERS_TABLE, COMMANDS_TABLE, AI_CONFIG_TABLE, SYNC_CONFIG_TABLE
};
use redb::{ReadableTable};
use crate::redis_manager::RedisConfig;

use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit, aead::Aead};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;


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

fn derive_aes_key(password: &str) -> Vec<u8> {
    let mut key = [0u8; 32];
    let salt = b"ssh_sync_secure_salt_2026_v1";
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key.to_vec()
}

fn encrypt_json(data: &str, master_key: &str) -> Result<Vec<u8>, String> {
    let key_bytes = derive_aes_key(master_key);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce = Nonce::from_slice(b"ssh_sync_vec");

    cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("加密过程出错: {}", e))
}

fn decrypt_json(encrypted_data: &[u8], master_key: &str) -> Result<String, String> {
    let key_bytes = derive_aes_key(master_key);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"ssh_sync_vec");

    let decrypted = cipher.decrypt(nonce, encrypted_data)
        .map_err(|_| "解密失败：主加密密钥错误或数据已被损坏".to_string())?;

    String::from_utf8(decrypted).map_err(|e| format!("编码转换失败: {}", e))
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
                    sc.master_key = String::new();
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
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };

        serde_json::to_string(&sync_data).map_err(|e| e.to_string())
    }

    pub async fn import_all_data(&self, json_str: String) -> Result<(), String> {
        let data: FullSyncData = serde_json::from_str(&json_str).map_err(|_| "云端备份文件格式无效".to_string())?;
        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;

        {
            let mut s_table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
            for s in data.servers {
                let val = serde_json::to_string(&s).unwrap();
                s_table.insert(s.id.as_str(), val.as_str()).map_err(|e| e.to_string())?;
            }

            let mut c_table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
            for c in data.commands {
                let val = serde_json::to_string(&c).unwrap();
                c_table.insert(c.id.as_str(), val.as_str()).map_err(|e| e.to_string())?;
            }

            if let Some(ai) = data.ai_config {
                let mut a_table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                let val = serde_json::to_string(&ai).unwrap();
                a_table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
            }

            let mut r_table = write_txn.open_table(crate::REDIS_CONN_TABLE).map_err(|e| e.to_string())?;
            for r in data.redis_configs {
                let id = r.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let val = serde_json::to_string(&r).unwrap();
                r_table.insert(id.as_str(), val.as_str()).map_err(|e| e.to_string())?;
            }

            if let Some(sc) = data.sync_config {
                let mut sync_table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
                let val = serde_json::to_string(&sc).unwrap();
                sync_table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
            }
        }

        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[command]
pub async fn save_sync_settings(state: State<'_, AppState>, mut config: SyncConfig) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
        config.master_key = String::new();
        let val = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;

    if let Some(v) = table.get("default").map_err(|e| e.to_string())? {
        let config: SyncConfig = serde_json::from_str(v.value()).map_err(|e| e.to_string())?;
        return Ok(config);
    }

    Ok(SyncConfig {
        endpoint: "".into(),
        username: "".into(),
        password: "".into(),
        remote_filename: "ssh_sync_backup.enc".into(),
        master_key: "".into(),
        auto_sync: false,
    })
}

#[command]
pub async fn sync_to_cloud(state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() { return Err("必须输入主加密密钥".into()); }

    let data_json = state.export_all_data().await?;

    let encrypted_bytes = encrypt_json(&data_json, &config.master_key)?;

    let client = reqwest::Client::new();
    let url = format!("{}/{}", config.endpoint.trim_end_matches('/'), config.remote_filename);

    let res = client.put(url)
        .basic_auth(&config.username, Some(&config.password))
        .body(encrypted_bytes)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    if res.status().is_success() {
        Ok("加密同步成功".into())
    } else {
        Err(format!("服务器错误 ({}): 请检查 WebDAV 账号密码", res.status()))
    }
}

#[command]
pub async fn sync_from_cloud(app_handle: tauri::AppHandle, state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() { return Err("必须输入主加密密钥以进行解密".into()); }

    let client = reqwest::Client::new();
    let url = format!("{}/{}", config.endpoint.trim_end_matches('/'), config.remote_filename);

    let res = client.get(url)
        .basic_auth(&config.username, Some(&config.password))
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    if res.status().is_success() {
        let encrypted_bytes = res.bytes().await.map_err(|e| e.to_string())?;

        let json_str = decrypt_json(&encrypted_bytes, &config.master_key)?;

        state.import_all_data(json_str).await?;
        use tauri::Emitter;
        app_handle.emit("database-changed", "sync").map_err(|e: tauri::Error| e.to_string())?;
        Ok("解密并同步成功".into())
    } else {
        Err(format!("下载失败: 状态码 {}", res.status()))
    }
}