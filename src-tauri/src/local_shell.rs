use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Runtime, Window};
use tokio::sync::Mutex as AsyncMutex;

pub const LOCAL_SERVER_ID: &str = "__local__";

#[derive(Serialize, Clone)]
struct ShellOutputPayload {
    server_id: String,
    session_id: String,
    data: String,
}

pub struct LocalSession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
}

pub type LocalSessionMap = Arc<AsyncMutex<HashMap<String, LocalSession>>>;

pub fn local_shell_label() -> &'static str {
    #[cfg(windows)]
    {
        "Windows PowerShell"
    }
    #[cfg(target_os = "macos")]
    {
        "Terminal"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "Shell"
    }
    #[cfg(not(any(windows, unix)))]
    {
        "Shell"
    }
}

fn build_shell_command() -> CommandBuilder {
    #[cfg(windows)]
    {
        let mut cmd = CommandBuilder::new("powershell.exe");
        cmd.args(["-NoLogo", "-NoProfile"]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            #[cfg(target_os = "macos")]
            {
                "/bin/zsh".to_string()
            }
            #[cfg(not(target_os = "macos"))]
            {
                "/bin/bash".to_string()
            }
        });
        CommandBuilder::new(shell)
    }
}

struct LocalShellHandles {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    reader: Box<dyn Read + Send>,
}

fn open_local_shell_process(cols: u16, rows: u16) -> Result<LocalShellHandles, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("打开本地 PTY 失败: {}", e))?;

    let cmd = build_shell_command();
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("启动本地 Shell 失败: {}", e))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("读取本地 Shell 输出失败: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("写入本地 Shell 失败: {}", e))?;

    Ok(LocalShellHandles {
        writer: Arc::new(Mutex::new(writer)),
        master: Arc::new(Mutex::new(pair.master)),
        child: Arc::new(Mutex::new(child)),
        reader,
    })
}

pub async fn spawn_local_shell<R: Runtime>(
    window: Window<R>,
    session_id: String,
    sessions: LocalSessionMap,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let cols = cols.max(40);
    let rows = rows.max(8);
    let handles = tokio::task::spawn_blocking(move || open_local_shell_process(cols, rows))
        .await
        .map_err(|e| format!("启动本地 Shell 任务失败: {}", e))??;

    let writer = handles.writer.clone();
    let master = handles.master.clone();
    let child = handles.child.clone();

    sessions.lock().await.insert(
        session_id.clone(),
        LocalSession {
            writer,
            master,
            child: child.clone(),
        },
    );

    let output_window = window.clone();
    let output_session_id = session_id.clone();
    std::thread::spawn(move || {
        let mut reader = handles.reader;
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = output_window.emit(
                        "ssh-output",
                        ShellOutputPayload {
                            server_id: LOCAL_SERVER_ID.to_string(),
                            session_id: output_session_id.clone(),
                            data: text,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });

    let wait_window = window;
    let wait_session_id = session_id;
    std::thread::spawn(move || {
        if let Ok(mut child) = child.lock() {
            let _ = child.wait();
        }
        let _ = wait_window.emit(
            "ssh-closed",
            serde_json::json!({
                "server_id": LOCAL_SERVER_ID,
                "session_id": wait_session_id,
            }),
        );
    });

    Ok(())
}

pub async fn write_local_shell(
    sessions: &LocalSessionMap,
    session_id: &str,
    data: String,
) -> Result<(), String> {
    let writer = {
        let map = sessions.lock().await;
        map.get(session_id)
            .ok_or_else(|| "Session not found".to_string())?
            .writer
            .clone()
    };

    tokio::task::spawn_blocking(move || {
        let mut guard = writer
            .lock()
            .map_err(|_| "本地 Shell 写入锁失败".to_string())?;
        guard
            .write_all(data.as_bytes())
            .map_err(|e| format!("写入本地 Shell 失败: {}", e))?;
        guard
            .flush()
            .map_err(|e| format!("刷新本地 Shell 失败: {}", e))
    })
    .await
    .map_err(|e| format!("本地 Shell 写入任务失败: {}", e))?
}

pub async fn resize_local_shell(
    sessions: &LocalSessionMap,
    session_id: &str,
    rows: u32,
    cols: u32,
) -> Result<(), String> {
    let master = {
        let map = sessions.lock().await;
        map.get(session_id)
            .ok_or_else(|| "Session not found".to_string())?
            .master
            .clone()
    };

    tokio::task::spawn_blocking(move || {
        let master = master
            .lock()
            .map_err(|_| "本地 Shell 尺寸锁失败".to_string())?;
        master
            .resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("调整本地 Shell 尺寸失败: {}", e))
    })
    .await
    .map_err(|e| format!("本地 Shell 尺寸任务失败: {}", e))?
}

pub async fn disconnect_local_shell(sessions: &LocalSessionMap, session_id: &str) -> Result<(), String> {
    let session = {
        let mut map = sessions.lock().await;
        map.remove(session_id)
    };

    if let Some(session) = session {
        tokio::task::spawn_blocking(move || {
            if let Ok(mut child) = session.child.lock() {
                let _ = child.kill();
                let _ = child.wait();
            }
        })
        .await
        .map_err(|e| format!("关闭本地 Shell 失败: {}", e))?;
    }

    Ok(())
}
