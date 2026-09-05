use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use harbor_acp::session::{AcpHostSession, ResumeKind};
use harbor_acp::spawn::SpawnSpec;
use harbor_core::SqlitePool;
use harbor_core::types::{ContentPart, DetectedEngine};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use crate::security::{ExecutableAllowlist, ExecutableKind};

#[derive(Clone, Default)]
pub struct AcpRegistry(Arc<Mutex<HashMap<String, AcpHostSession>>>);

pub fn grant_engines(allow: &ExecutableAllowlist, engines: &[DetectedEngine]) {
    for engine in engines {
        if engine.path.is_empty() {
            continue;
        }
        let _ = allow.grant(Path::new(&engine.path), ExecutableKind::Engine);
    }
}

fn acp_command(engine_id: &str) -> Result<(PathBuf, Vec<String>), String> {
    let spec = harbor_core::engines::catalog()
        .into_iter()
        .find(|spec| spec.id == engine_id)
        .ok_or_else(|| format!("unknown engine {engine_id}"))?;
    let path_env = std::env::var("PATH").unwrap_or_default();
    let cwd = std::env::current_dir().ok();
    let binary = if spec.chat_mode == "adapter" && spec.binaries.len() > 1 {
        spec.binaries.get(1)
    } else {
        spec.binaries.first()
    }
    .ok_or_else(|| format!("{engine_id} has no binary"))?;
    let command = harbor_core::engines::resolve_on_path(binary, &path_env, cwd.as_deref())
        .ok_or_else(|| format!("{engine_id} CLI is not on PATH"))?;
    Ok((command, spec.acp_args.unwrap_or_default()))
}

fn prompt_parts(parts: &[ContentPart]) -> Vec<Value> {
    parts
        .iter()
        .map(|part| {
            json!({
                "type": part.r#type,
                "text": part.text,
                "path": part.path
            })
        })
        .collect()
}

fn chunk_text(notes: &[Value]) -> String {
    notes
        .iter()
        .filter_map(|note| {
            let params = note.get("params")?;
            let content = params.get("content")?;
            content.get("text")?.as_str().map(str::to_string)
        })
        .collect::<Vec<_>>()
        .join("")
}

pub async fn prompt(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    thread_id: &str,
    parts: &[ContentPart],
) -> Result<(), String> {
    let ctx = harbor_core::threads::context(pool, thread_id)
        .await
        .map_err(|error| error.to_string())?;
    let (command, args) = acp_command(&ctx.engine_id)?;
    let granted = allow
        .grant(&command, ExecutableKind::Engine)
        .and_then(|path| allow.authorize(&path, ExecutableKind::Engine))
        .map_err(|error| error.to_string())?;
    let cwd = ctx
        .workspace_folder
        .clone()
        .filter(|folder| !folder.is_empty())
        .unwrap_or_else(|| std::env::temp_dir().display().to_string());
    let extra = ctx.extra_roots.clone();
    let stored = ctx.acp_session.clone();
    let engine_id = ctx.engine_id.clone();
    let prompt_parts = prompt_parts(parts);
    let registry = registry.clone();
    let registry_for_err = registry.clone();
    let thread_key = thread_id.to_string();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let mut sessions = registry.0.lock().map_err(|_| "acp registry".to_string())?;
        if sessions.get(&thread_key).is_none() {
            let spec = SpawnSpec {
                engine_id: engine_id.clone(),
                command: granted.display().to_string(),
                args,
                cwd,
                mcp_servers: vec![],
            };
            let mut session =
                AcpHostSession::connect(spec.clone()).map_err(|error| error.to_string())?;
            let kind = session
                .open_session(stored, &spec, &extra)
                .map_err(|error| error.to_string())?;
            let _ = session.take_notifications();
            session.resume_kind = kind;
            sessions.insert(thread_key.clone(), session);
        }
        let session = sessions
            .get_mut(&thread_key)
            .ok_or_else(|| "acp session".to_string())?;
        let result = session
            .prompt(&prompt_parts)
            .map_err(|error| error.to_string())?;
        let notes = session.take_notifications();
        Ok::<_, String>((
            result,
            notes,
            session.session_id.clone(),
            session.resume_kind,
            session.caps.config_options.clone(),
        ))
    })
    .await
    .map_err(|error| error.to_string())?;
    let (result, notes, session_id, kind, config_options) = match outcome {
        Ok(value) => value,
        Err(error) => {
            let _ = registry_for_err
                .0
                .lock()
                .map(|mut sessions| sessions.remove(thread_id));
            if error.contains("auth-required") {
                let _ = app.emit(
                    "engine_auth_required",
                    json!({ "engineId": ctx.engine_id, "hint": "CLI login" }),
                );
            }
            return Err(error);
        }
    };
    if let Some(session_id) = session_id.as_deref() {
        harbor_core::threads::set_acp_session(pool, thread_id, session_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    let mut prose = chunk_text(&notes);
    if prose.is_empty() {
        prose = result
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }
    if matches!(kind, ResumeKind::FreshWithBanner) && !prose.contains("could not be resumed") {
        let banner = "Previous session could not be resumed. Started a new session.";
        prose = if prose.is_empty() {
            banner.to_string()
        } else {
            format!("{banner}\n{prose}")
        };
    }
    if !prose.is_empty() {
        harbor_core::threads::append_message(pool, thread_id, "thread", "assistant", &prose)
            .await
            .map_err(|error| error.to_string())?;
    }
    let _ = app.emit(
        "acp_update",
        json!({
            "sessionRef": thread_id,
            "payload": {
                "text": prose,
                "stopReason": result.get("stopReason"),
                "configOptions": config_options
            }
        }),
    );
    Ok(())
}

pub async fn cancel(registry: &AcpRegistry, thread_id: &str) -> Result<(), String> {
    let registry = registry.clone();
    let thread_id = thread_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let mut sessions = registry.0.lock().map_err(|_| "acp registry".to_string())?;
        if let Some(session) = sessions.get_mut(&thread_id) {
            session.cancel().map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn drop_session(registry: &AcpRegistry, thread_id: &str) {
    if let Ok(mut sessions) = registry.0.lock() {
        sessions.remove(thread_id);
    }
}
