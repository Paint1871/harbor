use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use harbor_core::SqlitePool;
use harbor_paths::{ShellKind, quote_for_shell};
use harbor_pty::LivePty;
use tauri::{AppHandle, Emitter, State};

use crate::security::{ExecutableAllowlist, ExecutableKind};

#[derive(Default)]
pub struct PtyRegistry(Mutex<HashMap<String, LivePty>>);

use harbor_core::b64::encode as b64_encode;

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

#[cfg(unix)]
fn shell_from_preference(preference: Option<&str>) -> Option<PathBuf> {
    match preference {
        Some("zsh") => Some(PathBuf::from("/bin/zsh")),
        Some("bash") => Some(PathBuf::from("/bin/bash")),
        _ => None,
    }
}

#[cfg(unix)]
fn launch_via_shell(
    target: &Path,
    target_args: &[String],
    cwd: &Path,
) -> Result<(PathBuf, Vec<String>), String> {
    // Start from a stable system directory, then switch to the verified
    // workspace from inside the child. This avoids inheriting a macOS fork
    // state while zsh is resolving the renderer-provided working directory.
    let quoted_cwd = quote_for_shell(cwd, ShellKind::Unix).map_err(|error| error.to_string())?;
    let quoted_target =
        quote_for_shell(target, ShellKind::Unix).map_err(|error| error.to_string())?;
    let mut command = format!("cd -- {quoted_cwd} && exec {quoted_target}");
    for arg in target_args {
        command.push(' ');
        command.push_str(
            &quote_for_shell(Path::new(arg), ShellKind::Unix).map_err(|error| error.to_string())?,
        );
    }
    Ok((PathBuf::from("/"), vec!["-i".into(), "-c".into(), command]))
}

async fn workspace_root(
    pool: &SqlitePool,
    pane_id: &str,
    workspace_id: &str,
) -> Result<PathBuf, String> {
    let folder: Option<(String,)> = sqlx::query_as(
        "SELECT w.folder
         FROM panes p
         JOIN workspace_tabs t ON t.id = p.tab_id
         JOIN workspaces w ON w.id = t.workspace_id
         WHERE p.id = ?1 AND p.kind = 'terminal' AND t.workspace_id = ?2",
    )
    .bind(pane_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    let folder = folder.ok_or_else(|| "terminal pane is not part of this workspace".to_string())?;
    let root = PathBuf::from(folder.0);
    if !root.is_absolute() {
        return Err("workspace path is not absolute".into());
    }
    // Workspace paths are normalized when they enter the database. Do not
    // canonicalize here: on macOS, a renderer process without Documents
    // privacy access can block inside getcwd/opendir indefinitely. The child
    // starts from `/` and performs the quoted `cd` in shell_launch instead;
    // an unavailable folder then becomes a normal PTY exit that the UI can
    // report and retry.
    Ok(root)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn pty_spawn(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    registry: State<'_, PtyRegistry>,
    allow: State<'_, ExecutableAllowlist>,
    pane_id: String,
    workspace_id: String,
    cols: u16,
    rows: u16,
    shell: Option<String>,
    engine_id: Option<String>,
) -> Result<(), String> {
    if pane_id.trim().is_empty() || workspace_id.trim().is_empty() {
        return Err("pane and workspace are required".into());
    }
    let cwd = workspace_root(&pool, &pane_id, &workspace_id).await?;
    let shells = allow.granted(ExecutableKind::LoginShell);
    let saved_shell = harbor_core::settings::get(&pool, "default_shell")
        .await
        .ok()
        .flatten()
        .and_then(|value| value.as_str().map(str::to_owned));
    let configured_shell = match shell {
        Some(path) => PathBuf::from(path),
        None => {
            #[cfg(unix)]
            if let Some(path) = shell_from_preference(saved_shell.as_deref()) {
                path
            } else {
                shells
                    .first()
                    .cloned()
                    .ok_or_else(|| "no login shell is granted".to_string())?
            }
            #[cfg(not(unix))]
            {
                let _ = saved_shell;
                shells
                    .first()
                    .cloned()
                    .ok_or_else(|| "no login shell is granted".to_string())?
            }
        }
    };
    let login_shell = allow
        .authorize(&configured_shell, ExecutableKind::LoginShell)
        .map_err(|error| error.to_string())?;
    let requested_engine = engine_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty() && *id != "shell");
    // Names the terminal in a way the builder recognises in the inbox.
    let engine_label;
    let (target, target_args) = if let Some(engine_id) = requested_engine {
        let spec = harbor_core::engines::catalog()
            .into_iter()
            .find(|spec| spec.id == engine_id && spec.supports_terminal)
            .ok_or_else(|| format!("{engine_id} is not a supported terminal CLI"))?;
        let detected = harbor_core::engines::recheck()
            .into_iter()
            .find(|engine| {
                engine.id == engine_id
                    && engine.supports_terminal
                    && engine.status != "cli-missing"
                    && !engine.path.trim().is_empty()
            })
            .ok_or_else(|| {
                format!(
                    "{engine_id} is not installed or is no longer available; check installed CLIs again"
                )
            })?;
        let target = allow
            .grant(Path::new(&detected.path), ExecutableKind::Engine)
            .map_err(|error| error.to_string())?;
        engine_label = spec.display_name.clone();
        (target, spec.pty_args)
    } else {
        engine_label = "Shell".into();
        (login_shell.clone(), vec!["-i".into()])
    };
    let cols = cols.clamp(20, 500);
    let rows = rows.clamp(4, 200);
    if let Some(previous) = registry
        .0
        .lock()
        .map_err(|_| "pty registry".to_string())?
        .remove(&pane_id)
    {
        let _ = previous.kill();
    }
    #[cfg(unix)]
    let (launch_cwd, launch_args) = launch_via_shell(&target, &target_args, cwd.as_path())?;
    #[cfg(not(unix))]
    let (launch_cwd, launch_args) = (cwd.clone(), target_args);
    #[cfg(not(unix))]
    let mut spawn_allowlist = shells.clone();
    #[cfg(not(unix))]
    if !spawn_allowlist.iter().any(|path| path == &target) {
        spawn_allowlist.push(target.clone());
    }
    #[cfg(unix)]
    let spawn_program = login_shell.clone();
    #[cfg(not(unix))]
    let spawn_program = target;
    #[cfg(unix)]
    let spawn_allowlist = shells.clone();
    let (pty, rx) = LivePty::spawn_with_args(
        &spawn_program,
        &launch_args,
        launch_cwd.as_path(),
        cols,
        rows,
        &spawn_allowlist,
    )
    .map_err(|error| error.to_string())?;
    let emit_id = pane_id.clone();
    let notify_pool = (*pool).clone();
    let notify_workspace = workspace_id.clone();
    let notify_label = engine_label.clone();
    thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let _ = app.emit(
                "pty-data",
                serde_json::json!({ "paneId": emit_id, "b64": b64_encode(&chunk) }),
            );
        }
        let _ = app.emit("pty-exit", serde_json::json!({ "paneId": emit_id }));
        // A terminal in a workspace the builder is not looking at just ended.
        tauri::async_runtime::block_on(crate::ipc::notify(
            &app,
            &notify_pool,
            "terminal-exit",
            &format!("{notify_label} stopped"),
            "The terminal exited. Resume it to start a fresh shell.",
            harbor_core::notifications::Target::code(&notify_workspace, &emit_id),
        ));
    });
    // Insert only after the PTY has been created so a failed spawn never
    // leaves a stale registry entry behind.
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
    let cols = cols.clamp(20, 500);
    let rows = rows.clamp(4, 200);
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
pub fn pty_pause(registry: State<PtyRegistry>, pane_id: String) -> Result<(), String> {
    let guard = registry.0.lock().map_err(|_| "pty registry".to_string())?;
    if let Some(pty) = guard.get(&pane_id) {
        pty.pause().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resume(registry: State<PtyRegistry>, pane_id: String) -> Result<(), String> {
    let guard = registry.0.lock().map_err(|_| "pty registry".to_string())?;
    if let Some(pty) = guard.get(&pane_id) {
        pty.resume().map_err(|error| error.to_string())?;
    }
    Ok(())
}
