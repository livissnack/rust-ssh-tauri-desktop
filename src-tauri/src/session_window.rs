use crate::app_state::AppState;
use crate::local_shell::LOCAL_SERVER_ID;
use crate::servers::collect_servers_json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
struct SessionWindowBootstrap {
    server_id: String,
    session_id: String,
    session_name: String,
    #[serde(default)]
    is_local: bool,
}

#[tauri::command]
pub async fn open_session_window(
    app: AppHandle,
    state: State<'_, AppState>,
    server_id: String,
    base_name: String,
) -> Result<String, String> {
    let is_local = server_id == LOCAL_SERVER_ID;
    let session_id = if is_local {
        format!("local-{}", &Uuid::new_v4().simple().to_string()[..8])
    } else {
        format!(
            "{}-{}",
            server_id,
            &Uuid::new_v4().simple().to_string()[..8]
        )
    };
    let session_name = format!("{} (窗口)", base_name.trim());
    let label = format!(
        "session-{}",
        session_id.replace(['/', '\\', ':', ' '], "-")
    );

    if app.get_webview_window(&label).is_some() {
        return Err("该会话窗口已打开".into());
    }

    let bootstrap = SessionWindowBootstrap {
        server_id: server_id.clone(),
        session_id: session_id.clone(),
        session_name: session_name.clone(),
        is_local,
    };
    let bootstrap_json = serde_json::to_string(&bootstrap).map_err(|e| e.to_string())?;
    let servers_json = collect_servers_json(&state.db);
    let init_script = format!(
        "window.__SESSION_BOOTSTRAP__ = {}; window.__INITIAL_SERVERS__ = {};",
        bootstrap_json, servers_json
    );

    let url = if cfg!(debug_assertions) {
        WebviewUrl::External(
            "http://localhost:1420"
                .parse()
                .map_err(|e| format!("无效的 dev URL: {}", e))?,
        )
    } else {
        WebviewUrl::App("index.html".into())
    };

    let window = WebviewWindowBuilder::new(&app, &label, url)
        .title(&session_name)
        .inner_size(1200.0, 800.0)
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;

    let win = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    });

    Ok(session_id)
}

pub fn preheat_servers(window: &tauri::WebviewWindow, db: &Arc<redb::Database>) {
    let servers_json = collect_servers_json(db);
    let _ = window.eval(&format!("window.__INITIAL_SERVERS__ = {};", servers_json));
}
