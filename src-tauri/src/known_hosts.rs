use redb::{Database, ReadableTable, TableDefinition};
use russh::keys::{HashAlg, PublicKey, PublicKeyBase64};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

pub const KNOWN_HOSTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("known_hosts");

/// How long the UI may take to accept or reject a host key prompt.
pub const HOST_KEY_PROMPT_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HostKeyRole {
    Direct,
    Jump,
    Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHostRecord {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
    pub key_type: String,
    pub public_key: String,
    pub trusted_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum HostKeyPromptPayload {
    New {
        request_id: String,
        host_role: HostKeyRole,
        host: String,
        port: u16,
        server_name: String,
        fingerprint: String,
        key_type: String,
    },
    Changed {
        request_id: String,
        host_role: HostKeyRole,
        host: String,
        port: u16,
        server_name: String,
        fingerprint: String,
        old_fingerprint: String,
        key_type: String,
    },
}

pub fn host_lookup_key(host: &str, port: u16) -> String {
    format!("{}:{}", host.trim(), port)
}

pub fn public_key_fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

pub fn encode_public_key(key: &PublicKey) -> String {
    key.public_key_base64()
}

fn public_key_algorithm(key: &PublicKey) -> String {
    key.algorithm().to_string()
}

pub fn read_known_host(db: &Database, lookup: &str) -> Result<Option<KnownHostRecord>, String> {
    let read_txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(KNOWN_HOSTS_TABLE).map_err(|e| e.to_string())?;
    let Some(value) = table.get(lookup).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    serde_json::from_str(value.value()).map_err(|e| e.to_string()).map(Some)
}

pub fn write_known_host(db: &Database, record: &KnownHostRecord) -> Result<(), String> {
    let lookup = host_lookup_key(&record.host, record.port);
    let write_txn = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(KNOWN_HOSTS_TABLE).map_err(|e| e.to_string())?;
        table
            .insert(lookup.as_str(), serde_json::to_string(record).map_err(|e| e.to_string())?.as_str())
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())
}

pub fn delete_known_host(db: &Database, lookup: &str) -> Result<(), String> {
    let write_txn = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn.open_table(KNOWN_HOSTS_TABLE).map_err(|e| e.to_string())?;
        let _ = table.remove(lookup).map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())
}

pub struct HostKeyPromptHub {
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl HostKeyPromptHub {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }
}

pub struct HostKeyVerifier {
    db: Arc<Database>,
    app: AppHandle,
    hub: Arc<HostKeyPromptHub>,
    host: String,
    port: u16,
    server_name: String,
    allow_prompt: bool,
    window_label: Option<String>,
    host_role: HostKeyRole,
}

impl HostKeyVerifier {
    pub fn new(
        db: Arc<Database>,
        app: AppHandle,
        hub: Arc<HostKeyPromptHub>,
        host: String,
        port: u16,
        server_name: String,
        allow_prompt: bool,
        window_label: Option<String>,
        host_role: HostKeyRole,
    ) -> Self {
        Self {
            db,
            app,
            hub,
            host,
            port,
            server_name,
            allow_prompt,
            window_label,
            host_role,
        }
    }

    pub async fn verify(&self, key: &PublicKey) -> Result<bool, String> {
        let lookup = host_lookup_key(&self.host, self.port);
        let fingerprint = public_key_fingerprint(key);
        let key_type = public_key_algorithm(key);
        let public_key = encode_public_key(key);

        match read_known_host(&self.db, &lookup)? {
            Some(record) if record.fingerprint == fingerprint => Ok(true),
            Some(record) => {
                if !self.allow_prompt {
                    return Err(format!(
                        "Host key mismatch for {}:{} (expected {}, got {})",
                        self.host, self.port, record.fingerprint, fingerprint
                    ));
                }
                let trust = self
                    .prompt_changed(&record.fingerprint, &fingerprint, &key_type)
                    .await?;
                if trust {
                    self.save(&fingerprint, &key_type, &public_key)?;
                }
                Ok(trust)
            }
            None => {
                if !self.allow_prompt {
                    return Err(format!(
                        "Unknown host key for {}:{} — connect once to trust this host",
                        self.host, self.port
                    ));
                }
                let trust = self.prompt_new(&fingerprint, &key_type).await?;
                if trust {
                    self.save(&fingerprint, &key_type, &public_key)?;
                }
                Ok(trust)
            }
        }
    }

    fn save(&self, fingerprint: &str, key_type: &str, public_key: &str) -> Result<(), String> {
        let trusted_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        write_known_host(
            &self.db,
            &KnownHostRecord {
                host: self.host.clone(),
                port: self.port,
                fingerprint: fingerprint.to_string(),
                key_type: key_type.to_string(),
                public_key: public_key.to_string(),
                trusted_at,
            },
        )
    }

    async fn prompt_new(&self, fingerprint: &str, key_type: &str) -> Result<bool, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.hub.pending.lock().await.insert(request_id.clone(), tx);

        let payload = HostKeyPromptPayload::New {
            request_id: request_id.clone(),
            host_role: self.host_role,
            host: self.host.clone(),
            port: self.port,
            server_name: self.server_name.clone(),
            fingerprint: fingerprint.to_string(),
            key_type: key_type.to_string(),
        };
        self.emit_host_key_prompt(&payload)?;

        self.await_response(request_id, rx).await
    }

    async fn prompt_changed(
        &self,
        old_fingerprint: &str,
        fingerprint: &str,
        key_type: &str,
    ) -> Result<bool, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.hub.pending.lock().await.insert(request_id.clone(), tx);

        let payload = HostKeyPromptPayload::Changed {
            request_id: request_id.clone(),
            host_role: self.host_role,
            host: self.host.clone(),
            port: self.port,
            server_name: self.server_name.clone(),
            fingerprint: fingerprint.to_string(),
            old_fingerprint: old_fingerprint.to_string(),
            key_type: key_type.to_string(),
        };
        self.emit_host_key_prompt(&payload)?;

        self.await_response(request_id, rx).await
    }

    fn emit_host_key_prompt(&self, payload: &HostKeyPromptPayload) -> Result<(), String> {
        if let Some(label) = &self.window_label {
            self.app
                .emit_to(label, "ssh-host-key-prompt", payload)
                .map_err(|e| e.to_string())?;
        } else {
            self.app
                .emit("ssh-host-key-prompt", payload)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn await_response(
        &self,
        request_id: String,
        rx: oneshot::Receiver<bool>,
    ) -> Result<bool, String> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(HOST_KEY_PROMPT_TIMEOUT_SECS),
            rx,
        )
        .await;

        self.hub.pending.lock().await.remove(&request_id);

        match result {
            Ok(Ok(trusted)) => Ok(trusted),
            Ok(Err(_)) => Err("Host key prompt cancelled".to_string()),
            Err(_) => Err("Host key verification timed out".to_string()),
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn respond_host_key_prompt(
    state: tauri::State<'_, crate::AppState>,
    request_id: String,
    trust: bool,
) -> Result<(), String> {
    let tx = state
        .host_key_hub
        .pending
        .lock()
        .await
        .remove(&request_id)
        .ok_or("Host key prompt expired or not found")?;
    let _ = tx.send(trust);
    Ok(())
}

#[tauri::command]
pub async fn list_known_hosts(state: tauri::State<'_, crate::AppState>) -> Result<Vec<KnownHostRecord>, String> {
    let read_txn = state.db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn.open_table(KNOWN_HOSTS_TABLE).map_err(|e| e.to_string())?;
    let mut rows: Vec<KnownHostRecord> = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        rows.push(serde_json::from_str(value.value()).map_err(|e| e.to_string())?);
    }
    rows.sort_by(|a, b| a.host.cmp(&b.host).then(a.port.cmp(&b.port)));
    Ok(rows)
}

#[tauri::command]
pub async fn remove_known_host(
    state: tauri::State<'_, crate::AppState>,
    host: String,
    port: u16,
) -> Result<(), String> {
    delete_known_host(&state.db, &host_lookup_key(&host, port))
}
