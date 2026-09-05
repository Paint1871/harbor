use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use harbor_pty::LivePty;
use tauri::{AppHandle, Emitter, State};

use crate::security::{ExecutableAllowlist, ExecutableKind};

#[derive(Default)]
pub struct PtyRegistry(Mutex<HashMap<String, LivePty>>);

fn b64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(TABLE[((triple >> 12) & 63) as usize] as char);
        out.push(if i + 1 < bytes.len() {
            TABLE[((triple >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if i + 2 < bytes.len() {
            TABLE[(triple & 63) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    out
}

fn b64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(ch: u8) -> Option<u8> {
        match ch {
            b'A'..=b'Z' => Some(ch - b'A'),
            b'a'..=b'z' => Some(ch - b'a' + 26),
            b'0'..=b'9' => Some(ch - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let filtered: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !filtered.len().is_multiple_of(4) {
        return Err("invalid base64".into());
    }
    let mut out = Vec::new();
    for chunk in filtered.chunks(4) {
        let n = chunk.iter().filter(|b| **b != b'=').count();
        let v0 = val(chunk[0]).ok_or("invalid base64")?;
        let v1 = val(chunk[1]).ok_or("invalid base64")?;
        let v2 = if n > 2 { val(chunk[2]).unwrap_or(0) } else { 0 };
        let v3 = if n > 3 { val(chunk[3]).unwrap_or(0) } else { 0 };
        let triple = ((v0 as u32) << 18) | ((v1 as u32) << 12) | ((v2 as u32) << 6) | v3 as u32;
        out.push((triple >> 16) as u8);
        if n > 2 {
            out.push((triple >> 8) as u8);
        }
        if n > 3 {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    registry: State<PtyRegistry>,
    allow: State<ExecutableAllowlist>,
    pane_id: String,
    cwd: String,
    shell: Option<String>,
) -> Result<(), String> {
    let shells = allow.granted(ExecutableKind::LoginShell);
    let program = match shell {
        Some(path) => PathBuf::from(path),
        None => shells
            .first()
            .cloned()
            .ok_or_else(|| "no login shell is granted".to_string())?,
    };
    let granted = allow
        .authorize(&program, ExecutableKind::LoginShell)
        .map_err(|error| error.to_string())?;
    let cwd = {
        let given = PathBuf::from(&cwd);
        if given.is_absolute() {
            given
        } else {
            std::env::current_dir().unwrap_or(given)
        }
    };
    let (pty, rx) = LivePty::spawn(&granted, cwd.as_path(), 80, 24, &shells)
        .map_err(|error| error.to_string())?;
    let emit_id = pane_id.clone();
    thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let _ = app.emit(
                "pty-data",
                serde_json::json!({ "paneId": emit_id, "b64": b64_encode(&chunk) }),
            );
        }
    });
    registry
        .0
        .lock()
        .map_err(|_| "pty registry".to_string())?
        .insert(pane_id, pty);
    Ok(())
}

#[tauri::command]
pub fn pty_write_b64(
    registry: State<PtyRegistry>,
    pane_id: String,
    b64: String,
) -> Result<(), String> {
    let bytes = b64_decode(&b64)?;
    registry
        .0
        .lock()
        .map_err(|_| "pty registry".to_string())?
        .get(&pane_id)
        .ok_or_else(|| "pty not found".to_string())?
        .write(&bytes)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn pty_resize(
    registry: State<PtyRegistry>,
    pane_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    registry
        .0
        .lock()
        .map_err(|_| "pty registry".to_string())?
        .get(&pane_id)
        .ok_or_else(|| "pty not found".to_string())?
        .resize(cols, rows)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn pty_kill(registry: State<PtyRegistry>, pane_id: String) -> Result<(), String> {
    if let Some(pty) = registry
        .0
        .lock()
        .map_err(|_| "pty registry".to_string())?
        .remove(&pane_id)
    {
        pty.kill().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_pause(_pane_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn pty_resume(_pane_id: String) -> Result<(), String> {
    Ok(())
}
