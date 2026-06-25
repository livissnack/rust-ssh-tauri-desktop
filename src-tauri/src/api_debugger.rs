use crate::{AppState, API_DEBUGGER_TABLE};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const STORE_KEY: &str = "store";
const MAX_HISTORY: usize = 100;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiDebuggerStore {
    pub collections: Vec<ApiCollection>,
    pub environments: Vec<ApiEnvironment>,
    pub history: Vec<HistoryEntry>,
    pub active_env_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiCollection {
    pub id: String,
    pub name: String,
    pub requests: Vec<SavedRequest>,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub snapshot: HttpRequestSnapshot,
}

fn default_protocol() -> String {
    "http".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSnapshot {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub body: String,
    #[serde(default = "default_body_type", rename = "bodyType")]
    pub body_type: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub payload: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub sub_topic: String,
    #[serde(default)]
    pub pub_topic: String,
    #[serde(default)]
    pub pub_message: String,
}

fn default_body_type() -> String {
    "none".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiEnvironment {
    pub id: String,
    pub name: String,
    pub variables: Vec<EnvVariable>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EnvVariable {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: u64,
    pub snapshot: HttpRequestSnapshot,
    pub status: Option<u16>,
    pub elapsed_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct HiphupExport {
    format: String,
    version: u32,
    exported_at: u64,
    #[serde(flatten)]
    store: ApiDebuggerStore,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()[..9].to_string()
}

fn default_store() -> ApiDebuggerStore {
    ApiDebuggerStore {
        environments: vec![ApiEnvironment {
            id: new_id(),
            name: "Default".into(),
            variables: vec![],
        }],
        active_env_id: None,
        ..Default::default()
    }
}

fn normalize_store(mut store: ApiDebuggerStore) -> ApiDebuggerStore {
    if store.environments.is_empty() {
        store.environments = default_store().environments;
    }
    if store.history.len() > MAX_HISTORY {
        store.history.truncate(MAX_HISTORY);
    }
    if store.active_env_id.is_none() {
        store.active_env_id = store.environments.first().map(|e| e.id.clone());
    }
    store
}

pub fn read_store(db: &redb::Database) -> Result<ApiDebuggerStore, String> {
    let read_txn = db.begin_read().map_err(|e| e.to_string())?;
    let table = read_txn
        .open_table(API_DEBUGGER_TABLE)
        .map_err(|e| e.to_string())?;
    if let Some(raw) = table.get(STORE_KEY).map_err(|e| e.to_string())? {
        let store: ApiDebuggerStore =
            serde_json::from_str(raw.value()).map_err(|e| format!("解析 API 数据失败: {}", e))?;
        return Ok(normalize_store(store));
    }
    Ok(default_store())
}

pub fn write_store(db: &redb::Database, store: &ApiDebuggerStore) -> Result<(), String> {
    let store = normalize_store(store.clone());
    let write_txn = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_txn
            .open_table(API_DEBUGGER_TABLE)
            .map_err(|e| e.to_string())?;
        table
            .insert(
                STORE_KEY,
                serde_json::to_string(&store).map_err(|e| e.to_string())?.as_str(),
            )
            .map_err(|e| e.to_string())?;
    }
    write_txn.commit().map_err(|e| e.to_string())
}

fn merge_store(base: ApiDebuggerStore, incoming: ApiDebuggerStore) -> ApiDebuggerStore {
    let mut collections = base.collections;
    for collection in incoming.collections {
        if let Some(existing) = collections.iter_mut().find(|c| c.id == collection.id) {
            *existing = collection;
        } else {
            collections.push(collection);
        }
    }

    let mut environments = base.environments;
    for env in incoming.environments {
        if let Some(existing) = environments.iter_mut().find(|e| e.id == env.id) {
            *existing = env;
        } else {
            environments.push(env);
        }
    }

    let mut history = base.history;
    for entry in incoming.history {
        if history.iter().any(|h| h.id == entry.id) {
            continue;
        }
        history.push(entry);
    }
    history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    history.truncate(MAX_HISTORY);

    ApiDebuggerStore {
        collections,
        environments,
        history,
        active_env_id: incoming.active_env_id.or(base.active_env_id),
    }
}

fn body_type_from_postman(mode: &str, raw: &str, language: Option<&str>) -> (String, String) {
    match mode {
        "raw" => {
            if language == Some("json") {
                ("json".into(), raw.to_string())
            } else {
                ("text".into(), raw.to_string())
            }
        }
        "urlencoded" => ("form".into(), raw.to_string()),
        _ => ("none".into(), String::new()),
    }
}

fn postman_url_to_string(url: &serde_json::Value) -> String {
    match url {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => obj
            .get("raw")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn snapshot_from_postman_request(name: &str, request: &serde_json::Value) -> Option<SavedRequest> {
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_string();
    let url = request
        .get("url")
        .map(postman_url_to_string)
        .unwrap_or_default();
    let mut headers = Vec::new();
    if let Some(list) = request.get("header").and_then(|v| v.as_array()) {
        for item in list {
            headers.push(HeaderEntry {
                key: item.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                value: item.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                enabled: !item.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
            });
        }
    }

    let mut body = String::new();
    let mut body_type = "none".to_string();
    if let Some(body_obj) = request.get("body").and_then(|v| v.as_object()) {
        let mode = body_obj
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("raw");
        match mode {
            "urlencoded" => {
                body_type = "form".into();
                if let Some(items) = body_obj.get("urlencoded").and_then(|v| v.as_array()) {
                    body = items
                        .iter()
                        .filter_map(|item| {
                            let key = item.get("key").and_then(|v| v.as_str())?;
                            let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            Some(format!("{}={}", key, value))
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
            "raw" => {
                let raw = body_obj.get("raw").and_then(|v| v.as_str()).unwrap_or("");
                let language = body_obj
                    .get("options")
                    .and_then(|v| v.get("raw"))
                    .and_then(|v| v.get("language"))
                    .and_then(|v| v.as_str());
                let mapped = body_type_from_postman("raw", raw, language);
                body_type = mapped.0;
                body = mapped.1;
            }
            _ => {}
        }
    }

    let description = request
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(SavedRequest {
        id: new_id(),
        name: name.to_string(),
        description,
        snapshot: HttpRequestSnapshot {
            protocol: default_protocol(),
            method,
            url,
            headers,
            body,
            body_type,
            ..Default::default()
        },
    })
}

fn flatten_postman_items(items: &serde_json::Value, prefix: &str, out: &mut Vec<SavedRequest>) {
    let Some(list) = items.as_array() else {
        return;
    };
    for item in list {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("Request");
        let full_name = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };
        if item.get("request").is_some() {
            if let Some(saved) = snapshot_from_postman_request(&full_name, item.get("request").unwrap()) {
                out.push(saved);
            }
        }
        if let Some(nested) = item.get("item") {
            flatten_postman_items(nested, &full_name, out);
        }
    }
}

fn import_postman_collection(json: &serde_json::Value) -> Result<Vec<ApiCollection>, String> {
    let name = json
        .get("info")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Collection")
        .to_string();
    let mut requests = Vec::new();
    if let Some(items) = json.get("item") {
        flatten_postman_items(items, "", &mut requests);
    }
    Ok(vec![ApiCollection {
        id: new_id(),
        name,
        requests,
        updated_at: now_ms(),
    }])
}

fn import_postman_environment(json: &serde_json::Value) -> Result<Vec<ApiEnvironment>, String> {
    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Environment")
        .to_string();
    let mut variables = Vec::new();
    if let Some(values) = json.get("values").and_then(|v| v.as_array()) {
        for item in values {
            variables.push(EnvVariable {
                id: new_id(),
                key: item.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                value: item.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                enabled: item.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            });
        }
    }
    Ok(vec![ApiEnvironment {
        id: new_id(),
        name,
        variables,
    }])
}

fn detect_import_format(json: &serde_json::Value) -> Result<&'static str, String> {
    if json.get("format").and_then(|v| v.as_str()) == Some("hiphup-api-debugger") {
        return Ok("hiphup");
    }
    if json.get("info")
        .and_then(|v| v.get("schema"))
        .and_then(|v| v.as_str())
        .map(|s| s.contains("postman.com/json/collection"))
        .unwrap_or(false)
        || json.get("item").is_some()
    {
        return Ok("postman-collection");
    }
    if json.get("values").is_some() && json.get("name").is_some() {
        return Ok("postman-environment");
    }
    Err("无法识别的导入文件格式".into())
}

fn parse_import_file(content: &str, format: &str) -> Result<(ApiDebuggerStore, String), String> {
    let json: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("JSON 解析失败: {}", e))?;
    let resolved = if format == "auto" {
        detect_import_format(&json)?.to_string()
    } else {
        format.to_string()
    };

    let store = match resolved.as_str() {
        "hiphup" => {
            if let Ok(export) = serde_json::from_value::<HiphupExport>(json.clone()) {
                normalize_store(export.store)
            } else {
                normalize_store(serde_json::from_value::<ApiDebuggerStore>(json).map_err(|e| e.to_string())?)
            }
        }
        "postman-collection" => ApiDebuggerStore {
            collections: import_postman_collection(&json)?,
            ..default_store()
        },
        "postman-environment" => ApiDebuggerStore {
            environments: import_postman_environment(&json)?,
            ..default_store()
        },
        _ => return Err("不支持的导入格式".into()),
    };

    Ok((store, resolved))
}

fn apply_import(
    base: ApiDebuggerStore,
    incoming: ApiDebuggerStore,
    format: &str,
    mode: &str,
) -> ApiDebuggerStore {
    if mode == "merge" {
        return merge_store(base, incoming);
    }

    match format {
        "hiphup" => normalize_store(incoming),
        "postman-collection" => ApiDebuggerStore {
            collections: incoming.collections,
            ..base
        },
        "postman-environment" => {
            let active_env_id = incoming
                .environments
                .first()
                .map(|env| env.id.clone())
                .or(base.active_env_id);
            ApiDebuggerStore {
                environments: incoming.environments,
                active_env_id,
                ..base
            }
        }
        _ => normalize_store(incoming),
    }
}

fn postman_body(snapshot: &HttpRequestSnapshot) -> Option<serde_json::Value> {
    match snapshot.body_type.as_str() {
        "none" => None,
        "json" => Some(serde_json::json!({
            "mode": "raw",
            "raw": snapshot.body,
            "options": { "raw": { "language": "json" } }
        })),
        "text" => Some(serde_json::json!({
            "mode": "raw",
            "raw": snapshot.body,
            "options": { "raw": { "language": "text" } }
        })),
        "form" => {
            let urlencoded: Vec<serde_json::Value> = snapshot
                .body
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        return None;
                    }
                    let idx = line.find('=').unwrap_or(line.len());
                    Some(serde_json::json!({
                        "key": &line[..idx],
                        "value": line.get(idx + 1..).unwrap_or(""),
                        "type": "text"
                    }))
                })
                .collect();
            Some(serde_json::json!({
                "mode": "urlencoded",
                "urlencoded": urlencoded
            }))
        }
        _ => None,
    }
}

fn postman_request_item(name: &str, description: &str, snapshot: &HttpRequestSnapshot) -> serde_json::Value {
    let headers: Vec<serde_json::Value> = snapshot
        .headers
        .iter()
        .map(|h| {
            serde_json::json!({
                "key": h.key,
                "value": h.value,
                "disabled": !h.enabled
            })
        })
        .collect();
    let mut request = serde_json::json!({
        "method": snapshot.method,
        "header": headers,
        "url": snapshot.url
    });
    if !description.is_empty() {
        request["description"] = serde_json::Value::String(description.to_string());
    }
    if let Some(body) = postman_body(snapshot) {
        request["body"] = body;
    }
    serde_json::json!({
        "name": name,
        "request": request
    })
}

fn export_postman_collection(store: &ApiDebuggerStore) -> Result<String, String> {
    let mut items = Vec::new();
    for collection in &store.collections {
        let requests: Vec<serde_json::Value> = collection
            .requests
            .iter()
            .filter(|req| req.snapshot.protocol == "http")
            .map(|req| postman_request_item(&req.name, &req.description, &req.snapshot))
            .collect();
        items.push(serde_json::json!({
            "name": collection.name,
            "item": requests
        }));
    }
    let export = serde_json::json!({
        "info": {
            "name": "Hiphup API Export",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items
    });
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

fn export_postman_environment(store: &ApiDebuggerStore) -> Result<String, String> {
    let env = store
        .environments
        .iter()
        .find(|e| Some(&e.id) == store.active_env_id.as_ref())
        .or_else(|| store.environments.first())
        .ok_or("没有可导出的环境")?;
    let values: Vec<serde_json::Value> = env
        .variables
        .iter()
        .map(|v| {
            serde_json::json!({
                "key": v.key,
                "value": v.value,
                "enabled": v.enabled
            })
        })
        .collect();
    let export = serde_json::json!({
        "name": env.name,
        "values": values
    });
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

fn export_hiphup(store: &ApiDebuggerStore) -> Result<String, String> {
    let export = HiphupExport {
        format: "hiphup-api-debugger".into(),
        version: 1,
        exported_at: now_ms(),
        store: store.clone(),
    };
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

fn export_content(store: &ApiDebuggerStore, format: &str) -> Result<String, String> {
    match format {
        "hiphup" => export_hiphup(store),
        "postman-collection" => export_postman_collection(store),
        "postman-environment" => export_postman_environment(store),
        _ => Err("不支持的导出格式".into()),
    }
}

#[tauri::command]
pub fn get_api_debugger_data(state: tauri::State<'_, AppState>) -> Result<ApiDebuggerStore, String> {
    read_store(&state.db)
}

#[tauri::command]
pub fn save_api_debugger_data(
    state: tauri::State<'_, AppState>,
    store: ApiDebuggerStore,
) -> Result<(), String> {
    write_store(&state.db, &store)
}

#[tauri::command]
pub fn export_api_debugger_file(
    state: tauri::State<'_, AppState>,
    path: String,
    format: String,
) -> Result<(), String> {
    let store = read_store(&state.db)?;
    let content = export_content(&store, &format)?;
    fs::write(Path::new(&path), content).map_err(|e| format!("写入文件失败: {}", e))
}

#[tauri::command]
pub fn import_api_debugger_file(
    state: tauri::State<'_, AppState>,
    path: String,
    format: String,
    mode: String,
) -> Result<ApiDebuggerStore, String> {
    let content = fs::read_to_string(Path::new(&path)).map_err(|e| format!("读取文件失败: {}", e))?;
    let (incoming, resolved_format) = parse_import_file(&content, &format)?;
    let store = apply_import(read_store(&state.db)?, incoming, &resolved_format, &mode);
    write_store(&state.db, &store)?;
    Ok(store)
}
