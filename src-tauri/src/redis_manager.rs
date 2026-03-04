use redis::{AsyncCommands, Client};
use serde::Deserialize; // 移除了未使用的 Serialize
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State; // 移除了未使用的 Runtime

// --- 数据结构 ---

#[derive(Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: i64,
}

pub struct RedisState {
    pub connection: Arc<Mutex<Option<redis::aio::MultiplexedConnection>>>,
}

// --- 指令实现 ---

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
    pattern: String,
    state: State<'_, RedisState>,
) -> Result<Vec<String>, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        let keys: Vec<String> = conn.keys(pattern).await.map_err(|e| e.to_string())?;
        return Ok(keys);
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
    state: State<'_, RedisState>,
) -> Result<String, String> {
    let mut lock = state.connection.lock().await;
    if let Some(ref mut conn) = *lock {
        // 显式指定返回值为 ()
        conn.set::<_, _, ()>(key, value).await.map_err(|e| e.to_string())?;
        Ok("Success".into())
    } else {
        Err("Redis 未连接".into())
    }
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
        // 关键修复：使用 turbofish 语法显式标注返回类型为 ()
        conn.rename::<_, _, ()>(&old_key, &new_key).await.map_err(|e| e.to_string())?;
        return Ok(format!("已将 '{}' 重命名为 '{}'", old_key, new_key));
    }
    Err("Redis 未连接".into())
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