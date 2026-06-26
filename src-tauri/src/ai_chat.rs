use crate::{AppState, AI_CHAT_SESSIONS_TABLE};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};

const RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiChatSession {
    pub id: String,
    pub title: String,
    pub messages: Vec<AiChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiChatSessionSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub message_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn retention_cutoff() -> u64 {
    now_ms().saturating_sub(RETENTION_MS)
}

fn session_preview(session: &AiChatSession) -> String {
    session
        .messages
        .iter()
        .rev()
        .find(|m| !m.content.trim().is_empty())
        .map(|m| {
            let text = m.content.replace('\n', " ").trim().to_string();
            if text.chars().count() > 80 {
                format!("{}…", text.chars().take(80).collect::<String>())
            } else {
                text
            }
        })
        .unwrap_or_default()
}

fn to_summary(session: &AiChatSession) -> AiChatSessionSummary {
    AiChatSessionSummary {
        id: session.id.clone(),
        title: session.title.clone(),
        preview: session_preview(session),
        message_count: session.messages.len(),
        created_at: session.created_at,
        updated_at: session.updated_at,
    }
}

/// Local-only: never included in WebDAV sync.
pub fn prune_expired_sessions(db: &redb::Database) -> Result<u32, String> {
    let cutoff = retention_cutoff();
    let write_txn = db.begin_write().map_err(|e| e.to_string())?;
    let mut removed = 0u32;
    {
        let mut table = write_txn
            .open_table(AI_CHAT_SESSIONS_TABLE)
            .map_err(|e| e.to_string())?;
        let stale_ids: Vec<String> = table
            .iter()
            .map_err(|e| e.to_string())?
            .filter_map(|res| {
                let (key, value) = res.ok()?;
                let session: AiChatSession = serde_json::from_str(value.value()).ok()?;
                if session.updated_at < cutoff {
                    Some(key.value().to_string())
                } else {
                    None
                }
            })
            .collect();
        for id in stale_ids {
            table.remove(id.as_str()).map_err(|e| e.to_string())?;
            removed += 1;
        }
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(removed)
}

#[tauri::command]
pub async fn list_ai_chat_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<AiChatSessionSummary>, String> {
    let _ = prune_expired_sessions(&state.db)?;
    let cutoff = retention_cutoff();
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn
        .open_table(AI_CHAT_SESSIONS_TABLE)
        .map_err(|e| e.to_string())?;

    let mut sessions = Vec::new();
    for res in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = res.map_err(|e| e.to_string())?;
        if let Ok(session) = serde_json::from_str::<AiChatSession>(value.value()) {
            if session.updated_at >= cutoff {
                sessions.push(to_summary(&session));
            }
        }
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

#[tauri::command]
pub async fn get_ai_chat_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<Option<AiChatSession>, String> {
    let cutoff = retention_cutoff();
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn
        .open_table(AI_CHAT_SESSIONS_TABLE)
        .map_err(|e| e.to_string())?;

    if let Some(raw) = table.get(session_id.as_str()).map_err(|e| e.to_string())? {
        let session: AiChatSession =
            serde_json::from_str(raw.value()).map_err(|e| format!("解析 AI 会话失败: {}", e))?;
        if session.updated_at >= cutoff {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn save_ai_chat_session(
    state: tauri::State<'_, AppState>,
    mut session: AiChatSession,
) -> Result<AiChatSessionSummary, String> {
    if session.id.trim().is_empty() {
        return Err("会话 ID 无效".into());
    }
    let now = now_ms();
    if session.created_at == 0 {
        session.created_at = now;
    }
    session.updated_at = now;
    if session.title.trim().is_empty() {
        session.title = "新对话".into();
    }

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn
            .open_table(AI_CHAT_SESSIONS_TABLE)
            .map_err(|e| e.to_string())?;
        table
            .insert(
                session.id.as_str(),
                serde_json::to_string(&session)
                    .map_err(|e| e.to_string())?
                    .as_str(),
            )
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;

    let _ = prune_expired_sessions(&state.db)?;
    Ok(to_summary(&session))
}

#[tauri::command]
pub async fn delete_ai_chat_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn
            .open_table(AI_CHAT_SESSIONS_TABLE)
            .map_err(|e| e.to_string())?;
        table
            .remove(session_id.as_str())
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn clear_ai_chat_sessions(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn
            .open_table(AI_CHAT_SESSIONS_TABLE)
            .map_err(|e| e.to_string())?;
        let ids: Vec<String> = table
            .iter()
            .map_err(|e| e.to_string())?
            .filter_map(|res| res.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for id in ids {
            table.remove(id.as_str()).map_err(|e| e.to_string())?;
        }
    }
    write_txn.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn prune_ai_chat_sessions(state: tauri::State<'_, AppState>) -> Result<u32, String> {
    prune_expired_sessions(&state.db)
}
