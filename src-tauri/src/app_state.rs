use crate::known_hosts::HostKeyPromptHub;
use crate::local_shell::LocalSessionMap;
use crate::p2p;
use crate::port_forward::PortForwardMap;
use crate::ssh_session::ClientHandler;
use redb::TableDefinition;
use russh::ChannelId;
use russh_sftp::client::SftpSession;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub const SERVERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ssh_servers");
pub const COMMANDS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("quick_commands");
pub const AI_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ai_settings");
pub const SYNC_CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sync_config");
pub const REDIS_CONN_TABLE: TableDefinition<&str, &str> = TableDefinition::new("redis_connections");
pub const P2P_MESSAGES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("p2p_messages");
pub const P2P_REMARKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("p2p_remarks");
pub const API_DEBUGGER_TABLE: TableDefinition<&str, &str> = TableDefinition::new("api_debugger");
pub const AI_CHAT_SESSIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("ai_chat_sessions");

pub struct ActiveSession {
    pub handle: Arc<Mutex<russh::client::Handle<ClientHandler<tauri::Wry>>>>,
    pub channel_id: ChannelId,
    pub channel: Arc<Mutex<russh::Channel<russh::client::Msg>>>,
    pub sftp: Arc<Mutex<Option<SftpSession>>>,
}

pub struct SyncRuntime {
    pub debounce_active: bool,
    pub sync_in_progress: bool,
    pub pending_push: bool,
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self {
            debounce_active: false,
            sync_in_progress: false,
            pending_push: false,
        }
    }
}

pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<String, ActiveSession>>>,
    pub local_sessions: LocalSessionMap,
    pub db: Arc<redb::Database>,
    pub cancelled_tasks: Arc<Mutex<HashSet<String>>>,
    pub paused_tasks: Arc<Mutex<HashSet<String>>>,
    pub p2p_sender: mpsc::UnboundedSender<p2p::P2PCommand>,
    pub sync_runtime: Arc<tokio::sync::Mutex<SyncRuntime>>,
    pub port_forwards: PortForwardMap,
    pub app_handle: tauri::AppHandle,
    pub host_key_hub: Arc<HostKeyPromptHub>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            local_sessions: self.local_sessions.clone(),
            db: self.db.clone(),
            cancelled_tasks: self.cancelled_tasks.clone(),
            paused_tasks: self.paused_tasks.clone(),
            p2p_sender: self.p2p_sender.clone(),
            sync_runtime: self.sync_runtime.clone(),
            port_forwards: self.port_forwards.clone(),
            app_handle: self.app_handle.clone(),
            host_key_hub: self.host_key_hub.clone(),
        }
    }
}
