use crate::app_state::AppState;
use russh_sftp::client::SftpSession;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tauri::{Emitter, State, Window};
use encoding_rs::{Encoding, GBK, UTF_8, WINDOWS_1252};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

#[derive(Serialize)]
pub(crate) struct FileInfo {
    name: String,
    is_dir: bool,
    size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileDetail {
    path: String,
    name: String,
    is_dir: bool,
    size: u64,
    permissions: Option<String>,
    permissions_text: Option<String>,
    modified_at: Option<u64>,
    accessed_at: Option<u64>,
    uid: Option<u32>,
    gid: Option<u32>,
    user: Option<String>,
    group: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    #[serde(rename = "taskId")]
    task_id: String,
    progress: u64,
}

fn format_rwx(mode: u32) -> String {
    let mode = mode & 0o777;
    let chars = ["---", "--x", "-w-", "-wx", "r--", "r-x", "rw-", "rwx"];
    format!(
        "{}{}{}",
        chars[((mode >> 6) & 7) as usize],
        chars[((mode >> 3) & 7) as usize],
        chars[(mode & 7) as usize]
    )
}

fn parse_octal_mode(mode: &str) -> Result<u32, String> {
    let trimmed = mode.trim();
    let digits = trimmed.strip_prefix('0').unwrap_or(trimmed);
    if digits.is_empty() || digits.len() > 4 || !digits.chars().all(|c| c.is_ascii_digit() && c < '8') {
        return Err("无效的权限格式，请使用八进制数如 755 或 0644".into());
    }
    u32::from_str_radix(digits, 8).map_err(|_| "无效的权限格式，请使用八进制数如 755 或 0644".into())
}

fn file_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn build_file_detail(
    path: String,
    is_dir: bool,
    size: u64,
    permissions: Option<u32>,
    modified_at: Option<u64>,
    accessed_at: Option<u64>,
    uid: Option<u32>,
    gid: Option<u32>,
    user: Option<String>,
    group: Option<String>,
) -> FileDetail {
    let permissions_text = permissions.map(format_rwx);
    let permissions = permissions.map(|mode| format!("{:o}", mode & 0o777));
    FileDetail {
        name: file_name_from_path(&path),
        path,
        is_dir,
        size,
        permissions,
        permissions_text,
        modified_at,
        accessed_at,
        uid,
        gid,
        user,
        group,
    }
}

async fn create_sftp_session(state: &AppState, session_id: &str) -> Result<SftpSession, String> {
    let handle_mutex = {
        let sessions = state.sessions.lock().await;
        sessions
            .get(session_id)
            .ok_or("Session not found")?
            .handle
            .clone()
    };

    let channel = {
        let handle = handle_mutex.lock().await;
        handle
            .channel_open_session()
            .await
            .map_err(|e| e.to_string())?
    };
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;

    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("SFTP 初始化失败: {}", e))
}

async fn ensure_sftp_session(state: &AppState, session_id: &str) -> Result<(), String> {
    let sftp_arc = {
        let sessions = state.sessions.lock().await;
        let sess = sessions.get(session_id).ok_or("Session not found")?;
        if sess.sftp.lock().await.is_some() {
            return Ok(());
        }
        sess.sftp.clone()
    };

    let mut guard = sftp_arc.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let session = create_sftp_session(state, session_id).await?;
    *guard = Some(session);
    Ok(())
}

async fn sftp_arc_for(
    state: &AppState,
    session_id: &str,
) -> Result<Arc<Mutex<Option<SftpSession>>>, String> {
    ensure_sftp_session(state, session_id).await?;
    let sessions = state.sessions.lock().await;
    sessions
        .get(session_id)
        .ok_or_else(|| "Session not found".to_string())
        .map(|sess| sess.sftp.clone())
}

macro_rules! with_sftp {
    ($state:expr, $session_id:expr, |$sftp:ident| $body:expr) => {{
        let sftp_arc = sftp_arc_for($state, $session_id).await?;
        let guard = sftp_arc.lock().await;
        let $sftp = guard
            .as_ref()
            .ok_or_else(|| "SFTP session unavailable".to_string())?;
        $body
    }};
}

const MAX_SFTP_EDIT_BYTES: u64 = 2 * 1024 * 1024;

fn resolve_encoding(name: &str) -> Result<&'static Encoding, String> {
    match name.trim().to_lowercase().as_str() {
        "utf-8" | "utf8" => Ok(UTF_8),
        "gbk" | "gb2312" | "gb18030" => Ok(GBK),
        "latin1" | "iso-8859-1" | "windows-1252" | "cp1252" => Ok(WINDOWS_1252),
        other => Err(format!("Unsupported encoding: {other}")),
    }
}

fn decode_text_file(bytes: &[u8], encoding: &str) -> Result<String, String> {
    if bytes.contains(&0) {
        return Err("Binary file cannot be edited as text".into());
    }
    let enc = resolve_encoding(encoding)?;
    let (decoded, _, had_errors) = enc.decode(bytes);
    if had_errors && enc == UTF_8 {
        return Err("File is not valid UTF-8 text".into());
    }
    Ok(decoded.into_owned())
}

fn encode_text_file(content: &str, encoding: &str) -> Result<Vec<u8>, String> {
    let enc = resolve_encoding(encoding)?;
    let (encoded, _, had_errors) = enc.encode(content);
    if had_errors {
        return Err(format!(
            "Content contains characters that cannot be encoded as {encoding}"
        ));
    }
    Ok(encoded.into_owned())
}

fn ensure_editable_size(size: u64) -> Result<(), String> {
    if size > MAX_SFTP_EDIT_BYTES {
        return Err(format!(
            "File too large to edit (max {} MB)",
            MAX_SFTP_EDIT_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
}

async fn read_limited<R: AsyncReadExt + Unpin>(
    mut reader: R,
    max_bytes: u64,
    known_size: Option<u64>,
) -> Result<Vec<u8>, String> {
    if let Some(size) = known_size {
        ensure_editable_size(size)?;
    }
    let mut buffer = Vec::new();
    let mut chunk = vec![0u8; 65536];
    loop {
        let n = reader.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if buffer.len() as u64 + n as u64 > max_bytes {
            return Err(format!(
                "File too large to edit (max {} MB)",
                max_bytes / (1024 * 1024)
            ));
        }
        buffer.extend_from_slice(&chunk[..n]);
    }
    Ok(buffer)
}

#[tauri::command]
pub async fn read_local_file(path: String, encoding: Option<String>) -> Result<String, String> {
    let enc = encoding.unwrap_or_else(|| "utf-8".to_string());
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    if meta.is_dir() {
        return Err("Cannot edit a directory".into());
    }
    ensure_editable_size(meta.len())?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    decode_text_file(&bytes, &enc)
}

#[tauri::command]
pub async fn write_local_file(
    path: String,
    content: String,
    encoding: Option<String>,
) -> Result<(), String> {
    let enc = encoding.unwrap_or_else(|| "utf-8".to_string());
    let bytes = encode_text_file(&content, &enc)?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| format!("Failed to save file: {}", e))
}

#[tauri::command]
pub async fn read_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    encoding: Option<String>,
) -> Result<String, String> {
    let enc = encoding.unwrap_or_else(|| "utf-8".to_string());
    with_sftp!(state.inner(), &session_id, |sftp| {
        let meta = sftp.metadata(&path).await.map_err(|e| e.to_string())?;
        if meta.is_dir() {
            return Err("Cannot edit a directory".into());
        }
        ensure_editable_size(meta.len())?;
        let mut file = sftp.open(&path).await.map_err(|e| e.to_string())?;
        let bytes = read_limited(&mut file, MAX_SFTP_EDIT_BYTES, Some(meta.len())).await?;
        decode_text_file(&bytes, &enc)
    })
}

#[tauri::command]
pub async fn write_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    content: String,
    encoding: Option<String>,
) -> Result<(), String> {
    let enc = encoding.unwrap_or_else(|| "utf-8".to_string());
    let bytes = encode_text_file(&content, &enc)?;
    with_sftp!(state.inner(), &session_id, |sftp| {
        let mut file = sftp.create(&path).await.map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to save file: {}", e))?;
        Ok(())
    })
}

#[tauri::command]
pub async fn list_local_dir(path: String) -> Result<Vec<FileInfo>, String> {
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    files.push(FileInfo {
        name: "..".to_string(),
        is_dir: true,
        size: 0,
    });
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(meta) = entry.metadata() {
                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_dir: meta.is_dir(),
                    size: meta.len(),
                });
            }
        }
    }
    files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(files)
}

#[tauri::command]
pub async fn get_local_file_info(path: String) -> Result<FileDetail, String> {
    let meta = tokio::fs::metadata(&path).await.map_err(|e| e.to_string())?;
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let accessed_at = meta
        .accessed()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        Some(meta.permissions().mode())
    };
    #[cfg(not(unix))]
    let permissions: Option<u32> = None;

    Ok(build_file_detail(
        path,
        meta.is_dir(),
        meta.len(),
        permissions,
        modified_at,
        accessed_at,
        None,
        None,
        None,
        None,
    ))
}

#[tauri::command]
pub async fn rename_local_file(old_path: String, new_path: String) -> Result<(), String> {
    tokio::fs::rename(&old_path, &new_path)
        .await
        .map_err(|e| format!("重命名失败: {}", e))
}

#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("路径不存在".into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if path.is_dir() {
            Command::new("explorer")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        } else {
            let selected = path
                .to_str()
                .ok_or_else(|| "路径包含无效字符".to_string())?;
            Command::new("explorer")
                .args(["/select,", selected])
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let path_str = path
            .to_str()
            .ok_or_else(|| "路径包含无效字符".to_string())?;
        if path.is_file() {
            Command::new("open")
                .args(["-R", path_str])
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        } else {
            Command::new("open")
                .arg(path_str)
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let target = if path.is_file() {
            path.parent().unwrap_or(path.as_path())
        } else {
            path.as_path()
        };
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_remote_file_info(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<FileDetail, String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        let meta = sftp.metadata(&path).await.map_err(|e| e.to_string())?;
        Ok(build_file_detail(
            path,
            meta.is_dir(),
            meta.len(),
            meta.permissions,
            meta.mtime.map(u64::from),
            meta.atime.map(u64::from),
            meta.uid,
            meta.gid,
            meta.user,
            meta.group,
        ))
    })
}

#[tauri::command]
pub async fn rename_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        sftp.rename(old_path, new_path)
            .await
            .map_err(|e| format!("重命名失败: {}", e))
    })
}

#[tauri::command]
pub async fn set_remote_file_permissions(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    mode: String,
) -> Result<(), String> {
    let parsed_mode = parse_octal_mode(&mode)?;
    with_sftp!(state.inner(), &session_id, |sftp| {
        let mut meta = sftp.metadata(&path).await.map_err(|e| e.to_string())?;
        let current = meta
            .permissions
            .unwrap_or(if meta.is_dir() { 0o040755 } else { 0o100644 });
        let type_bits = current & !0o777;
        meta.permissions = Some(type_bits | (parsed_mode & 0o777));
        sftp.set_metadata(&path, meta)
            .await
            .map_err(|e| format!("修改权限失败: {}", e))
    })
}

#[tauri::command]
pub async fn delete_local_file(path: String, is_dir: bool) -> Result<(), String> {
    if is_dir {
        tokio::fs::remove_dir(&path)
            .await
            .map_err(|e| format!("删除目录失败: {}", e))
    } else {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| format!("删除文件失败: {}", e))
    }
}

async fn copy_dir_all(src: &Path, dest: &Path) -> Result<(), String> {
    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| format!("创建目录失败: {}", e))?;
    let mut entries = tokio::fs::read_dir(src)
        .await
        .map_err(|e| format!("读取目录失败: {}", e))?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("读取文件类型失败: {}", e))?;
        if file_type.is_dir() {
            Box::pin(copy_dir_all(&entry_path, &dest_path)).await?;
        } else {
            tokio::fs::copy(&entry_path, &dest_path)
                .await
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn create_local_path(path: String, is_dir: bool) -> Result<(), String> {
    if Path::new(&path).exists() {
        return Err("路径已存在".into());
    }
    if is_dir {
        tokio::fs::create_dir(&path)
            .await
            .map_err(|e| format!("创建文件夹失败: {}", e))
    } else {
        if let Some(parent) = Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建父目录失败: {}", e))?;
        }
        tokio::fs::File::create(&path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
pub async fn copy_local_path(src: String, dest: String) -> Result<(), String> {
    let src_path = Path::new(&src);
    if !src_path.exists() {
        return Err("源路径不存在".into());
    }
    if Path::new(&dest).exists() {
        return Err("目标已存在".into());
    }
    if src_path.is_dir() {
        copy_dir_all(src_path, Path::new(&dest)).await
    } else {
        if let Some(parent) = Path::new(&dest).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建父目录失败: {}", e))?;
        }
        tokio::fs::copy(&src, &dest)
            .await
            .map_err(|e| format!("复制失败: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
pub async fn move_local_path(src: String, dest: String, is_dir: bool) -> Result<(), String> {
    if Path::new(&dest).exists() {
        return Err("目标已存在".into());
    }
    if let Some(parent) = Path::new(&dest).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    match tokio::fs::rename(&src, &dest).await {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_local_path(src.clone(), dest.clone()).await?;
            if is_dir {
                tokio::fs::remove_dir_all(&src)
                    .await
                    .map_err(|e| format!("移动失败: {}", e))
            } else {
                tokio::fs::remove_file(&src)
                    .await
                    .map_err(|e| format!("移动失败: {}", e))
            }
        }
    }
}

async fn copy_remote_file_stream(sftp: &SftpSession, src: &str, dest: &str) -> Result<(), String> {
    let mut src_file = sftp.open(src).await.map_err(|e| e.to_string())?;
    let mut dest_file = sftp.create(dest).await.map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 65536];
    loop {
        let n = src_file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        dest_file
            .write_all(&buffer[..n])
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn copy_remote_recursive(sftp: &SftpSession, src: &str, dest: &str) -> Result<(), String> {
    let meta = sftp.metadata(src).await.map_err(|e| e.to_string())?;
    if meta.is_dir() {
        sftp.create_dir(dest).await.map_err(|e| e.to_string())?;
        for entry in sftp.read_dir(src).await.map_err(|e| e.to_string())? {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let sub_src = format!("{}/{}", src.trim_end_matches('/'), name);
            let sub_dest = format!("{}/{}", dest.trim_end_matches('/'), name);
            Box::pin(copy_remote_recursive(sftp, &sub_src, &sub_dest)).await?;
        }
    } else {
        copy_remote_file_stream(sftp, src, dest).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn create_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        if is_dir {
            sftp.create_dir(&path)
                .await
                .map_err(|e| format!("创建文件夹失败: {}", e))
        } else {
            sftp.create(&path)
                .await
                .map_err(|e| format!("创建文件失败: {}", e))?;
            Ok(())
        }
    })
}

#[tauri::command]
pub async fn copy_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    src: String,
    dest: String,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        copy_remote_recursive(sftp, &src, &dest).await
    })
}

#[tauri::command]
pub async fn move_remote_path(
    state: State<'_, AppState>,
    session_id: String,
    src: String,
    dest: String,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        sftp.rename(src, dest)
            .await
            .map_err(|e| format!("移动失败: {}", e))
    })
}

#[tauri::command]
pub async fn list_remote_dir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<Vec<FileInfo>, String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        let entries = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        files.push(FileInfo {
            name: "..".to_string(),
            is_dir: true,
            size: 0,
        });

        for entry in entries {
            let filename = entry.file_name();
            if filename == "." || filename == ".." {
                continue;
            }
            let metadata = entry.metadata();
            files.push(FileInfo {
                name: filename.to_string(),
                is_dir: metadata.is_dir(),
                size: metadata.size.unwrap_or(0),
            });
        }
        files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(files)
    })
}

async fn wait_transfer_tick(state: &AppState, task_id: &str) -> Result<(), String> {
    loop {
        {
            let cancelled = state.cancelled_tasks.lock().await;
            if cancelled.contains(task_id) {
                drop(cancelled);
                state.cancelled_tasks.lock().await.remove(task_id);
                state.paused_tasks.lock().await.remove(task_id);
                return Err("Task cancelled".into());
            }
        }
        {
            let paused = state.paused_tasks.lock().await;
            if !paused.contains(task_id) {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tauri::command]
pub async fn sftp_upload(
    window: Window,
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        let mut local_file = tokio::fs::File::open(&local_path)
            .await
            .map_err(|e| e.to_string())?;
        let total_size = local_file.metadata().await.map_err(|e| e.to_string())?.len();

        let mut remote_file = sftp.create(&remote_path).await.map_err(|e| e.to_string())?;

        let mut buffer = vec![0u8; 65536];
        let mut uploaded_size = 0u64;

        loop {
            wait_transfer_tick(state.inner(), &task_id).await?;

            let n = local_file.read(&mut buffer).await.map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }

            remote_file
                .write_all(&buffer[..n])
                .await
                .map_err(|e| e.to_string())?;
            uploaded_size += n as u64;

            let progress = ((uploaded_size as f64 / total_size as f64) * 100.0) as u64;
            let _ = window.emit(
                "transfer-progress",
                ProgressPayload {
                    task_id: task_id.clone(),
                    progress,
                },
            );
        }
        Ok(())
    })
}

#[tauri::command]
pub async fn sftp_download(
    window: Window,
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        let mut remote_file = sftp.open(&remote_path).await.map_err(|e| e.to_string())?;
        let metadata = remote_file.metadata().await.map_err(|e| e.to_string())?;
        let total_size = metadata.size.unwrap_or(0);

        let mut local_file = tokio::fs::File::create(&local_path)
            .await
            .map_err(|e| e.to_string())?;

        let mut buffer = vec![0u8; 65536];
        let mut downloaded_size = 0u64;

        loop {
            wait_transfer_tick(state.inner(), &task_id).await?;

            let n = remote_file.read(&mut buffer).await.map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..n])
                .await
                .map_err(|e| e.to_string())?;
            downloaded_size += n as u64;

            if total_size > 0 {
                let progress = ((downloaded_size as f64 / total_size as f64) * 100.0) as u64;
                let _ = window.emit(
                    "transfer-progress",
                    ProgressPayload {
                        task_id: task_id.clone(),
                        progress,
                    },
                );
            }
        }

        let _ = window.emit(
            "transfer-progress",
            ProgressPayload {
                task_id: task_id.clone(),
                progress: 100,
            },
        );

        Ok(())
    })
}

#[tauri::command]
pub async fn abort_transfer(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.cancelled_tasks.lock().await.insert(task_id.clone());
    state.paused_tasks.lock().await.remove(&task_id);
    Ok(())
}

#[tauri::command]
pub async fn pause_transfer(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.paused_tasks.lock().await.insert(task_id);
    Ok(())
}

#[tauri::command]
pub async fn resume_transfer(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.paused_tasks.lock().await.remove(&task_id);
    Ok(())
}

#[tauri::command]
pub async fn delete_remote_file(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    with_sftp!(state.inner(), &session_id, |sftp| {
        if is_dir {
            sftp.remove_dir(path)
                .await
                .map_err(|e| format!("删除目录失败: {}", e))
        } else {
            sftp.remove_file(path)
                .await
                .map_err(|e| format!("删除文件失败: {}", e))
        }
    })
}
