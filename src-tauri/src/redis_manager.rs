use crate::sync::{trigger_auto_sync};
use crate::security::{encrypt_secret, decrypt_secret};
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;
use crate::{AppState, REDIS_CONN_TABLE};
use uuid::Uuid;
use redb::{ReadableTable};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RedisConfig {
  pub id: Option<String>,
  #[serde(default = "default_conn_name")]
  pub name: String,
  pub host: String,
  pub port: u16,
  pub password: Option<String>,
  pub db: i64,
}

fn default_conn_name() -> String {
    "未命名连接".to_string()
}

pub struct RedisState {
    pub connection: Arc<Mutex<Option<redis::aio::MultiplexedConnection>>>,
}

#[tauri::command]
pub async fn redis_connect(
    config: RedisConfig,
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let password_part = config.password
        .filter(|p| !p.is_empty())
        .map(|p| format!(":{}@", p))
        .unwrap_or_default();

    let connection_string = format!(
        "redis://{}{}:{}/{}",
        password_part, config.host, config.port, config.db
    );

    let client = Client::open(connection_string).map_err(|e| e.to_string())?;

    let conn = client
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| e.to_string())?;

    let mut lock = state.connection.lock().await;
    *lock = Some(conn);

    Ok("Connected".into())
}

#[tauri::command]
pub async fn redis_get_keys(
    mut pattern: String,
    state: State<'_, RedisState>,
) -> Result<Vec<String>, String> {
    let mut lock = state.connection.lock().await;

    if let Some(ref mut conn) = *lock {
        if !pattern.is_empty() && pattern != "*" && !pattern.contains('*') {
            pattern = format!("*{}*", pattern);
        }

        if pattern.is_empty() {
            pattern = "*".to_string();
        }

        let keys: Vec<String> = conn.keys(pattern).await.map_err(|e| e.to_string())?;

        let mut sorted_keys = keys;
        sorted_keys.sort();

        return Ok(sorted_keys);
    }
    Err("Redis 未连接".into())
}

#[tauri::command]
pub async fn redis_get_value(
    key: String,
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        let key_type: String = redis::cmd("TYPE").arg(&key).query_async(conn).await.map_err(|e| e.to_string())?;

        if key_type == "string" {
            let val: Option<String> = conn.get(key).await.map_err(|e| e.to_string())?;
            return Ok(val.unwrap_or_default());
        }
        return Ok(format!("[Type: {}] 目前仅支持预览 String 类型", key_type));
    }
    Err("Redis 未连接".into())
}

#[tauri::command]
pub async fn redis_set_value(
    key: String,
    value: String,
    key_type: String,
    field: Option<String>,
    ttl: i64,
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let mut lock = state.connection.lock().await;

    let conn = lock.as_mut().ok_or("Redis 未连接")?;

    match key_type.as_str() {
        "string" => {
            conn.set::<_, _, ()>(&key, value).await.map_err(|e| e.to_string())?;
        }
        "hash" => {
            let f = field.ok_or("Hash 类型必须提供 Field 字段")?;
            if f.trim().is_empty() { return Err("Field 不能为空".into()); }
            conn.hset::<_, _, _, ()>(&key, f, value).await.map_err(|e| e.to_string())?;
        }
        "list" => {
            conn.rpush::<_, _, ()>(&key, value).await.map_err(|e| e.to_string())?;
        }
        "set" => {
            conn.sadd::<_, _, ()>(&key, value).await.map_err(|e| e.to_string())?;
        }
        _ => return Err(format!("暂不支持的类型: {}", key_type)),
    }

    if ttl > 0 {
        let _: () = conn.expire(&key, ttl).await.map_err(|e| e.to_string())?;
    } else if ttl == -1 {
        let _: () = conn.persist(&key).await.map_err(|e| e.to_string())?;
    }

    Ok("操作成功".into())
}

#[tauri::command]
pub async fn redis_del_key(
    key: String,
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        let _: i64 = conn.del(&key).await.map_err(|e| e.to_string())?;
        return Ok(format!("Key '{}' 已删除", key));
    }
    Err("Redis 未连接".into())
}

#[tauri::command]
pub async fn redis_rename_key(
    old_key: String,
    new_key: String,
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        conn.rename::<_, _, ()>(&old_key, &new_key).await.map_err(|e| e.to_string())?;
        return Ok(format!("已将 '{}' 重命名为 '{}'", old_key, new_key));
    }
    Err("Redis 未连接".into())
}

#[tauri::command]
pub async fn redis_get_type(key: String, state: State<'_, RedisState>) -> Result<String, String> {
    let mut lock = state.connection.lock().await;
    let conn = lock.as_mut().ok_or("Redis 未连接")?;
    let key_type: String = redis::cmd("TYPE").arg(&key).query_async(conn).await.map_err(|e| e.to_string())?;
    Ok(key_type)
}

#[tauri::command]
pub async fn redis_get_ttl(
    key: String,
    state: State<'_, RedisState>,
) -> Result<i64, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        let ttl: i64 = conn.ttl(&key).await.map_err(|e| e.to_string())?;
        return Ok(ttl);
    }
    Err("Redis 未连接".into())
}

#[tauri::command]
pub async fn save_redis_config(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    mut config: RedisConfig
) -> Result<RedisConfig, String> {
    if config.id.is_none() || config.id.as_ref().unwrap().is_empty() {
        config.id = Some(Uuid::new_v4().to_string());
    }
    let id = config.id.as_ref().unwrap().clone();

    // 加密 Host
    if !config.host.is_empty() {
        config.host = encrypt_secret(&config.host)?;
    }
    // 加密 Password (Redis 密码通常在 password 字段)
    if let Some(ref pass) = config.password {
        if !pass.is_empty() {
            config.password = Some(encrypt_secret(pass)?);
        }
    }

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(REDIS_CONN_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        table.insert(id.as_str(), json.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    // 触发同步
    trigger_auto_sync(state.inner(), app_handle.clone()).await;
    Ok(config)
}

#[tauri::command]
pub async fn get_redis_configs(state: State<'_, AppState>) -> Result<Vec<RedisConfig>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(REDIS_CONN_TABLE).map_err(|e| e.to_string())?;

    let mut configs = Vec::new();
    let iter = table.iter().map_err(|e| e.to_string())?;

    for result in iter {
        if let Ok((_key, value)) = result {
            match serde_json::from_str::<RedisConfig>(value.value()) {
                Ok(mut config) => {
                    // --- 执行解密 ---
                    // 解密 Host
                    if !config.host.is_empty() {
                        config.host = decrypt_secret(&config.host)
                            .unwrap_or_else(|_| "DECRYPT_ERROR".into());
                    }
                    // 解密 Password
                    if let Some(ref pass) = config.password {
                        config.password = Some(decrypt_secret(pass)
                            .unwrap_or_else(|_| "".into()));
                    }

                    configs.push(config);
                },
                Err(e) => eprintln!("跳过无效配置: {}", e),
            }
        }
    }
    Ok(configs)
}

#[tauri::command]
pub async fn delete_redis_config(app_handle: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(REDIS_CONN_TABLE).map_err(|e| e.to_string())?;
        table.remove(id.as_str()).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    // 触发同步
    trigger_auto_sync(state.inner(), app_handle).await;
    Ok(())
}

#[tauri::command]
pub async fn clear_all_redis_configs(state: State<'_, AppState>) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(REDIS_CONN_TABLE).map_err(|e| e.to_string())?;

        let keys: Vec<String> = table.iter()
            .map_err(|e| e.to_string())?
            .filter_map(|res| res.ok().map(|(k, _)| k.value().to_string()))
            .collect();

        for key in keys {
            table.remove(key.as_str()).map_err(|e| e.to_string())?;
        }
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}