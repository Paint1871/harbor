use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use harbor_acp::session::{AcpHostSession, ResumeKind};
use harbor_acp::spawn::SpawnSpec;
use harbor_acp::transport::write_message;
use harbor_acp::{PermissionHook, permission_outcome};
use harbor_core::SqlitePool;
use harbor_core::types::{ContentPart, DetectedEngine};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

use crate::security::{ExecutableAllowlist, ExecutableKind};

const PERMISSION_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionKind {
    Thread,
    Agent,
}

impl SessionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Thread => "thread",
            Self::Agent => "agent",
        }
    }
}

struct LiveSession {
    session: Mutex<AcpHostSession>,
    stdin: Arc<Mutex<ChildStdin>>,
    acp_id: Mutex<Option<String>>,
}

struct PendingPermission {
    session_ref: String,
    tx: std::sync::mpsc::Sender<Value>,
}

#[derive(Clone, Default)]
pub struct AcpRegistry {
    sessions: Arc<Mutex<HashMap<String, Arc<LiveSession>>>>,
    pending: Arc<Mutex<HashMap<String, PendingPermission>>>,
}

struct TurnContext {
    engine_id: String,
    cwd: String,
    extra_roots: Vec<String>,
    acp_session: Option<String>,
    kind: SessionKind,
}

pub fn grant_engines(allow: &ExecutableAllowlist, engines: &[DetectedEngine]) {
    for engine in engines {
        if engine.path.is_empty() {
            continue;
        }
        let _ = allow.grant(Path::new(&engine.path), ExecutableKind::Engine);
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn block_on<T>(fut: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Handle::current().block_on(fut)
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
            if note.get("method")?.as_str()? != "session/update" {
                return None;
            }
            let update = note.get("params")?.get("update")?;
            if update.get("sessionUpdate")?.as_str()? != "agent_message_chunk" {
                return None;
            }
            let content = update.get("content")?;
            content.get("text")?.as_str().map(str::to_string)
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn agent_cwd(home_path: &str, places: &[String]) -> String {
    if !home_path.trim().is_empty() {
        return home_path.to_string();
    }
    places
        .iter()
        .find(|path| !path.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().display().to_string())
}

async fn thread_turn_context(pool: &SqlitePool, thread_id: &str) -> Result<TurnContext, String> {
    let ctx = harbor_core::threads::context(pool, thread_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(TurnContext {
        engine_id: ctx.engine_id,
        cwd: ctx
            .workspace_folder
            .filter(|folder| !folder.is_empty())
            .unwrap_or_else(|| std::env::temp_dir().display().to_string()),
        extra_roots: ctx.extra_roots,
        acp_session: ctx.acp_session,
        kind: SessionKind::Thread,
    })
}

async fn agent_turn_context(pool: &SqlitePool, chat_id: &str) -> Result<TurnContext, String> {
    let ctx = harbor_core::chats::context(pool, chat_id)
        .await
        .map_err(|error| error.to_string())?;
    let cwd = agent_cwd(&ctx.home_path, &ctx.extra_dirs);
    let extra_roots = ctx
        .extra_dirs
        .into_iter()
        .filter(|path| path != &cwd && !path.trim().is_empty())
        .collect();
    Ok(TurnContext {
        engine_id: ctx.engine_id,
        cwd,
        extra_roots,
        acp_session: ctx.acp_session,
        kind: SessionKind::Agent,
    })
}

async fn persist_acp_session(
    pool: &SqlitePool,
    session_ref: &str,
    kind: SessionKind,
    session_id: &str,
) -> Result<(), String> {
    match kind {
        SessionKind::Thread => harbor_core::threads::set_acp_session(pool, session_ref, session_id)
            .await
            .map_err(|error| error.to_string()),
        SessionKind::Agent => harbor_core::chats::set_acp_session(pool, session_ref, session_id)
            .await
            .map_err(|error| error.to_string()),
    }
}

fn permission_card(id: &str, session_ref: &str, params: &Value) -> Value {
    let tool = params.get("toolCall").unwrap_or(params);
    let path = tool
        .get("path")
        .and_then(Value::as_str)
        .or_else(|| {
            tool.get("locations")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("path"))
                .and_then(Value::as_str)
        })
        .map(str::to_string);
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|option| {
            Some(json!({
                "optionId": option.get("optionId")?.as_str()?,
                "kind": option.get("kind")?.as_str()?,
                "name": option.get("name").and_then(Value::as_str).unwrap_or("")
            }))
        })
        .collect::<Vec<_>>();
    json!({
        "id": id,
        "sessionRef": session_ref,
        "title": tool.get("title").and_then(Value::as_str).unwrap_or("Permission"),
        "path": path,
        "command": tool.get("command").and_then(Value::as_str),
        "options": options
    })
}

fn make_permission_hook(
    app: AppHandle,
    pool: SqlitePool,
    registry: AcpRegistry,
    session_ref: String,
    kind: SessionKind,
) -> PermissionHook {
    Arc::new(move |params: Value| {
        let perm_id = uuid::Uuid::now_v7().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        if let Ok(mut pending) = registry.pending.lock() {
            pending.insert(
                perm_id.clone(),
                PendingPermission {
                    session_ref: session_ref.clone(),
                    tx,
                },
            );
        }
        let options_json = params
            .get("options")
            .cloned()
            .unwrap_or_else(|| json!([]))
            .to_string();
        let tool = params.get("toolCall").cloned().unwrap_or(Value::Null);
        let title = tool
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string);
        let path = tool.get("path").and_then(Value::as_str).map(str::to_string);
        let command = tool
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string);
        let _ = block_on(async {
            sqlx::query(
                "INSERT INTO acp_permissions
                 (id, session_ref, session_kind, tool_title, path, command, options_json, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
            )
            .bind(&perm_id)
            .bind(&session_ref)
            .bind(kind.as_str())
            .bind(&title)
            .bind(&path)
            .bind(&command)
            .bind(&options_json)
            .bind(now())
            .execute(&pool)
            .await
        });
        let _ = app.emit(
            "acp_permission",
            permission_card(&perm_id, &session_ref, &params),
        );
        match rx.recv_timeout(PERMISSION_TIMEOUT) {
            Ok(outcome) => Ok(outcome),
            Err(_) => {
                if let Ok(mut pending) = registry.pending.lock() {
                    pending.remove(&perm_id);
                }
                let _ = block_on(harbor_core::acp::permission_resolve(
                    &pool, &perm_id, None, true,
                ));
                let _ = cancel_sync(&registry, &session_ref);
                Ok(permission_outcome(None, true))
            }
        }
    })
}

fn cancel_sync(registry: &AcpRegistry, session_ref: &str) -> Result<(), String> {
    let live = {
        let sessions = registry
            .sessions
            .lock()
            .map_err(|_| "acp registry".to_string())?;
        sessions.get(session_ref).cloned()
    };
    let Some(live) = live else {
        return Ok(());
    };
    let session_id = live
        .acp_id
        .lock()
        .map_err(|_| "acp session".to_string())?
        .clone();
    let mut stdin = live.stdin.lock().map_err(|_| "acp stdin".to_string())?;
    write_message(
        &mut *stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": { "sessionId": session_id }
        }),
    )
    .map_err(|error| error.to_string())
}

fn get_or_connect(
    registry: &AcpRegistry,
    key: &str,
    ctx: &TurnContext,
    granted: &Path,
    args: Vec<String>,
    hook: PermissionHook,
) -> Result<Arc<LiveSession>, String> {
    {
        let sessions = registry
            .sessions
            .lock()
            .map_err(|_| "acp registry".to_string())?;
        if let Some(live) = sessions.get(key) {
            return Ok(live.clone());
        }
    }
    let spec = SpawnSpec {
        engine_id: ctx.engine_id.clone(),
        command: granted.display().to_string(),
        args,
        cwd: ctx.cwd.clone(),
        mcp_servers: vec![],
    };
    let mut session = AcpHostSession::connect(spec.clone()).map_err(|error| error.to_string())?;
    session.set_permission_hook(hook);
    let kind = session
        .open_session(ctx.acp_session.clone(), &spec, &ctx.extra_roots)
        .map_err(|error| error.to_string())?;
    let _ = session.take_notifications();
    session.resume_kind = kind;
    let stdin = session.stdin_handle();
    let acp_id = session.session_id.clone();
    let live = Arc::new(LiveSession {
        session: Mutex::new(session),
        stdin,
        acp_id: Mutex::new(acp_id),
    });
    let mut sessions = registry
        .sessions
        .lock()
        .map_err(|_| "acp registry".to_string())?;
    if let Some(existing) = sessions.get(key) {
        return Ok(existing.clone());
    }
    sessions.insert(key.to_string(), live.clone());
    Ok(live)
}

async fn run_turn(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    session_ref: &str,
    ctx: TurnContext,
    parts: &[ContentPart],
) -> Result<(), String> {
    let (command, args) = acp_command(&ctx.engine_id)?;
    let granted = allow
        .grant(&command, ExecutableKind::Engine)
        .and_then(|path| allow.authorize(&path, ExecutableKind::Engine))
        .map_err(|error| error.to_string())?;
    let prompt_parts = prompt_parts(parts);
    let registry = registry.clone();
    let registry_for_err = registry.clone();
    let session_key = session_ref.to_string();
    let hook = make_permission_hook(
        app.clone(),
        pool.clone(),
        registry.clone(),
        session_key.clone(),
        ctx.kind,
    );
    let persist_pool = pool.clone();
    let persist_kind = ctx.kind;
    let persist_ref = session_key.clone();
    let engine_id = ctx.engine_id.clone();
    let chat_kind = persist_kind.as_str();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let live = get_or_connect(&registry, &session_key, &ctx, &granted, args, hook)?;
        let acp_id = live
            .acp_id
            .lock()
            .map_err(|_| "acp session".to_string())?
            .clone();
        if let Some(session_id) = acp_id.as_deref() {
            let _ = block_on(persist_acp_session(
                &persist_pool,
                &persist_ref,
                persist_kind,
                session_id,
            ));
        }
        let mut session = live.session.lock().map_err(|_| "acp session".to_string())?;
        let result = session
            .prompt(&prompt_parts)
            .map_err(|error| error.to_string())?;
        let notes = session.take_notifications();
        if let Ok(mut stored) = live.acp_id.lock() {
            *stored = session.session_id.clone();
        }
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
            let _ = registry_for_err.sessions.lock().map(|mut sessions| {
                sessions.remove(session_ref);
            });
            if error.contains("auth-required") {
                let _ = app.emit(
                    "engine_auth_required",
                    json!({ "engineId": engine_id, "hint": "CLI login" }),
                );
            }
            return Err(error);
        }
    };
    if let Some(session_id) = session_id.as_deref() {
        persist_acp_session(pool, session_ref, persist_kind, session_id).await?;
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
        harbor_core::threads::append_message(pool, session_ref, chat_kind, "assistant", &prose)
            .await
            .map_err(|error| error.to_string())?;
    }
    let _ = app.emit(
        "acp_update",
        json!({
            "sessionRef": session_ref,
            "payload": {
                "text": prose,
                "stopReason": result.get("stopReason"),
                "configOptions": config_options
            }
        }),
    );
    Ok(())
}

pub async fn prompt(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    thread_id: &str,
    parts: &[ContentPart],
) -> Result<(), String> {
    let ctx = thread_turn_context(pool, thread_id).await?;
    run_turn(app, pool, allow, registry, thread_id, ctx, parts).await
}

pub async fn prompt_agent(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    chat_id: &str,
    parts: &[ContentPart],
) -> Result<(), String> {
    let ctx = agent_turn_context(pool, chat_id).await?;
    run_turn(app, pool, allow, registry, chat_id, ctx, parts).await
}

pub async fn cancel(registry: &AcpRegistry, session_ref: &str) -> Result<(), String> {
    let registry = registry.clone();
    let session_ref = session_ref.to_string();
    tauri::async_runtime::spawn_blocking(move || cancel_sync(&registry, &session_ref))
        .await
        .map_err(|error| error.to_string())?
}

pub fn drop_session(registry: &AcpRegistry, session_ref: &str) {
    if let Ok(mut sessions) = registry.sessions.lock() {
        sessions.remove(session_ref);
    }
    if let Ok(mut pending) = registry.pending.lock() {
        pending.retain(|_, item| item.session_ref != session_ref);
    }
}

pub async fn drop_agent_sessions(pool: &SqlitePool, registry: &AcpRegistry, agent_id: &str) {
    let chats = harbor_core::chats::list(pool, agent_id)
        .await
        .unwrap_or_default();
    for chat in chats {
        drop_session(registry, &chat.id);
    }
}

pub async fn set_live_config(
    registry: &AcpRegistry,
    session_ref: &str,
    option_id: String,
    value: Value,
) -> Result<(), String> {
    let registry = registry.clone();
    let session_ref = session_ref.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let live = {
            let sessions = registry
                .sessions
                .lock()
                .map_err(|_| "acp registry".to_string())?;
            sessions.get(&session_ref).cloned()
        };
        let Some(live) = live else {
            return Ok(());
        };
        let mut session = live.session.lock().map_err(|_| "acp session".to_string())?;
        session
            .set_config_option(&option_id, value)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub async fn resolve_permission(
    pool: &SqlitePool,
    registry: &AcpRegistry,
    id: &str,
    option_id: Option<String>,
    cancelled: bool,
) -> Result<(), String> {
    let outcome = permission_outcome(option_id.as_deref(), cancelled);
    let pending = {
        let mut pending = registry
            .pending
            .lock()
            .map_err(|_| "acp registry".to_string())?;
        pending.remove(id)
    };
    let session_ref = pending.as_ref().map(|item| item.session_ref.clone());
    if let Some(pending) = pending {
        let _ = pending.tx.send(outcome);
    }
    if cancelled && let Some(session_ref) = session_ref.as_deref() {
        let _ = cancel(registry, session_ref).await;
    }
    match harbor_core::acp::permission_resolve(pool, id, option_id.as_deref(), cancelled).await {
        Ok(()) => Ok(()),
        Err(_) if session_ref.is_some() => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harbor_core::types::CreateAgent;

    #[test]
    fn extracts_only_visible_assistant_chunks_from_v1_updates() {
        let notes = vec![
            json!({"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hello "}}}}),
            json!({"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"private thought"}}}}),
            json!({"method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"world"}}}}),
        ];
        assert_eq!(chunk_text(&notes), "Hello world");
    }

    #[test]
    fn agent_cwd_prefers_home_then_place_then_temp() {
        assert_eq!(agent_cwd("/home/agent", &["/place".into()]), "/home/agent");
        assert_eq!(agent_cwd("", &["/place".into()]), "/place");
        assert_eq!(
            agent_cwd("  ", &[]),
            std::env::temp_dir().display().to_string()
        );
    }

    #[tokio::test]
    async fn agent_chat_context_uses_home_and_places() {
        let dir = tempfile::tempdir().unwrap();
        let pool = harbor_core::db::open(&dir.path().join("db.sqlite"))
            .await
            .unwrap();
        let agent = harbor_core::agents::create(
            &pool,
            CreateAgent {
                name: "Mate".into(),
                brief: "brief".into(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        sqlx::query("UPDATE agents SET home_path = ?1 WHERE id = ?2")
            .bind(home.display().to_string())
            .bind(&agent.id)
            .execute(&pool)
            .await
            .unwrap();
        let extra = dir.path().join("extra");
        std::fs::create_dir_all(&extra).unwrap();
        sqlx::query("INSERT INTO places (id, agent_id, path, granted_at) VALUES (?1, ?2, ?3, ?4)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&agent.id)
            .bind(extra.display().to_string())
            .bind(now())
            .execute(&pool)
            .await
            .unwrap();
        let chat = harbor_core::chats::create(&pool, &agent.id).await.unwrap();
        harbor_core::chats::send(
            &pool,
            &chat.id,
            &[ContentPart {
                r#type: "text".into(),
                text: Some("hello agent".into()),
                path: None,
            }],
        )
        .await
        .unwrap();
        let ctx = agent_turn_context(&pool, &chat.id).await.unwrap();
        assert_eq!(ctx.engine_id, "opencode");
        assert!(ctx.cwd.ends_with("home"));
        assert_eq!(ctx.extra_roots.len(), 1);
        persist_acp_session(&pool, &chat.id, SessionKind::Agent, "sess-agent")
            .await
            .unwrap();
        let ctx = agent_turn_context(&pool, &chat.id).await.unwrap();
        assert_eq!(ctx.acp_session.as_deref(), Some("sess-agent"));
    }

    #[tokio::test]
    async fn permission_resolve_updates_row_and_channel() {
        let dir = tempfile::tempdir().unwrap();
        let pool = harbor_core::db::open(&dir.path().join("db.sqlite"))
            .await
            .unwrap();
        let registry = AcpRegistry::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let id = "perm-1".to_string();
        registry.pending.lock().unwrap().insert(
            id.clone(),
            PendingPermission {
                session_ref: "chat-1".into(),
                tx,
            },
        );
        sqlx::query(
            "INSERT INTO acp_permissions
             (id, session_ref, session_kind, tool_title, path, command, options_json, status, created_at)
             VALUES (?1, 'chat-1', 'agent', 'Run', NULL, NULL, '[]', 'pending', ?2)",
        )
        .bind(&id)
        .bind(now())
        .execute(&pool)
        .await
        .unwrap();
        resolve_permission(&pool, &registry, &id, Some("opt-allow".into()), false)
            .await
            .unwrap();
        let outcome = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(outcome["outcome"], "selected");
        assert_eq!(outcome["optionId"], "opt-allow");
        let (status, selected): (String, Option<String>) =
            sqlx::query_as("SELECT status, selected_option_id FROM acp_permissions WHERE id = ?1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "selected");
        assert_eq!(selected.as_deref(), Some("opt-allow"));
    }
}
