use crate::app_state::{AppState, COMMANDS_TABLE};
use crate::sync::schedule_push_sync;
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuickCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub group: Option<String>,
    pub updated_at: u64,
    #[serde(default)]
    pub deleted: bool,
}

#[tauri::command]
pub async fn get_quick_commands(state: State<'_, AppState>) -> Result<Vec<QuickCommand>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;

    let mut commands = Vec::new();
    for result in table.iter().map_err(|e| e.to_string())? {
        let (_key, value) = result.map_err(|e| e.to_string())?;
        let cmd: QuickCommand = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;

        if !cmd.deleted {
            commands.push(cmd);
        }
    }

    commands.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(commands)
}

#[tauri::command]
pub async fn save_quick_command(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mut cmd: QuickCommand,
) -> Result<QuickCommand, String> {
    if cmd.id.is_empty() {
        cmd.id = Uuid::new_v4().to_string();
    }

    cmd.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    cmd.deleted = false;

    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;

        table
            .insert(cmd.id.as_str(), json.as_str())
            .map_err(|e| e.to_string())?;
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;

    Ok(cmd)
}

#[tauri::command]
pub async fn delete_quick_command(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let write_txn = state.db.begin_write().map_err(|e| e.to_string())?;

    {
        let mut table = write_txn.open_table(COMMANDS_TABLE).map_err(|e| e.to_string())?;

        let existing_data = table
            .get(id.as_str())
            .map_err(|e| e.to_string())?
            .map(|v| v.value().to_string());

        if let Some(json_str) = existing_data {
            let mut cmd: QuickCommand = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

            cmd.deleted = true;
            cmd.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let json = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;

            table
                .insert(id.as_str(), json.as_str())
                .map_err(|e| e.to_string())?;
        }
    }

    write_txn.commit().map_err(|e| e.to_string())?;

    schedule_push_sync(state.inner(), app_handle).await;
    Ok(())
}
