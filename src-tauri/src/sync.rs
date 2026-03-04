use serde::{Deserialize, Serialize};
use tauri::{State, command};
use crate::{
    AppState, ServerConfig, QuickCommand, AiConfig,
    SERVERS_TABLE, COMMANDS_TABLE, AI_CONFIG_TABLE, SYNC_CONFIG_TABLE
};
use redb::{ReadableTable};

// 加密相关依赖
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit, aead::Aead};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

// --- 数据结构定义 ---

#[derive(Serialize, Deserialize)]
pub struct FullSyncData {
    pub servers: Vec<ServerConfig>,
    pub commands: Vec<QuickCommand>,
    pub ai_config: Option<AiConfig>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub endpoint: String,
    pub username: String,
    pub password: String,
    pub remote_filename: String,
    #[serde(default)] // 允许反序列化时缺失
    pub master_key: String,
    pub auto_sync: bool,
}

// --- 1. 加密辅助工具 ---

/// 使用 PBKDF2 从用户主密码派生出 32 字节的 AES 密钥
fn derive_aes_key(password: &str) -> Vec<u8> {
    let mut key = [0u8; 32];
    // 盐值建议固定，用于保证同一密码生成同一密钥
    let salt = b"ssh_sync_secure_salt_2026_v1";
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key.to_vec()
}

/// 加密 JSON 字符串为字节流
fn encrypt_json(data: &str, master_key: &str) -> Result<Vec<u8>, String> {
    let key_bytes = derive_aes_key(master_key);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // 实际生产建议使用随机 Nonce 并附加在密文头部，这里为了演示使用固定 Nonce
    let nonce = Nonce::from_slice(b"ssh_sync_vec");

    cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("加密过程出错: {}", e))
}

/// 解密字节流回 JSON 字符串
fn decrypt_json(encrypted_data: &[u8], master_key: &str) -> Result<String, String> {
    let key_bytes = derive_aes_key(master_key);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"ssh_sync_vec");

    let decrypted = cipher.decrypt(nonce, encrypted_data)
        .map_err(|_| "解密失败：主加密密钥错误或数据已被损坏".to_string())?;

    String::from_utf8(decrypted).map_err(|e| format!("编码转换失败: {}", e))
}

// --- 2. AppState 数据操作实现 ---

impl AppState {
    /// 导出当前所有数据库内容为 JSON
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

        let sync_data = FullSyncData {
            servers,
            commands,
            ai_config,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };

        serde_json::to_string(&sync_data).map_err(|e| e.to_string())
    }

    /// 将解析后的 JSON 写入本地数据库
    pub async fn import_all_data(&self, json_str: String) -> Result<(), String> {
        let data: FullSyncData = serde_json::from_str(&json_str).map_err(|_| "云端备份文件格式无效".to_string())?;
        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;

        {
            // 写入服务器
            let mut s_table = write_txn.open_table(SERVERS_TABLE).map_err(|e| e.to_string())?;
            for s in data.servers {
                let val = serde_json::to_string(&s).unwrap();
                s_table.insert(s.id.as_str(), val.as_str()).map_err(|e| e.to_string())?;
            }

            // 写入指令
            let mut c_table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
            for c in data.commands {
                let val = serde_json::to_string(&c).unwrap();
                c_table.insert(c.id.as_str(), val.as_str()).map_err(|e| e.to_string())?;
            }

            // 写入 AI 配置
            if let Some(ai) = data.ai_config {
                let mut a_table = write_txn.open_table(AI_CONFIG_TABLE).map_err(|e| e.to_string())?;
                let val = serde_json::to_string(&ai).unwrap();
                a_table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
            }
        }

        write_txn.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

// --- 3. 对外暴露给前端的 Tauri Commands ---

/// 保存 WebDAV 配置（不保存 master_key 以策安全）
#[command]
pub async fn save_sync_settings(state: State<'_, AppState>, mut config: SyncConfig) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;
        // 关键安全点：持久化时清除 master_key
        config.master_key = String::new();
        let val = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table.insert("default", val.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取已保存的 WebDAV 配置
#[command]
pub async fn get_sync_settings(state: State<'_, AppState>) -> Result<SyncConfig, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(SYNC_CONFIG_TABLE).map_err(|e| e.to_string())?;

    if let Some(v) = table.get("default").map_err(|e| e.to_string())? {
        let config: SyncConfig = serde_json::from_str(v.value()).map_err(|e| e.to_string())?;
        return Ok(config);
    }

    // 如果没有配置，返回一个全空的结构体，而不是 None
    Ok(SyncConfig {
        endpoint: "".into(),
        username: "".into(),
        password: "".into(),
        remote_filename: "ssh_sync_backup.enc".into(),
        master_key: "".into(),
        auto_sync: false,
    })
}

/// 加密并上传
#[command]
pub async fn sync_to_cloud(state: State<'_, AppState>, config: SyncConfig) -> Result<String, String> {
    if config.master_key.is_empty() { return Err("必须输入主加密密钥".into()); }

    // 1. 导出明文 JSON
    let data_json = state.export_all_data().await?;

    // 2. AES-256 加密
    let encrypted_bytes = encrypt_json(&data_json, &config.master_key)?;

    // 3. 发送请求
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

/// 下载并解密
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

        // 1. AES-256 解密
        let json_str = decrypt_json(&encrypted_bytes, &config.master_key)?;

        // 2. 导入数据库
        state.import_all_data(json_str).await?;
        use tauri::Emitter;
        app_handle.emit("database-changed", "sync").map_err(|e: tauri::Error| e.to_string())?;
        Ok("解密并同步成功".into())
    } else {
        Err(format!("下载失败: 状态码 {}", res.status()))
    }
}