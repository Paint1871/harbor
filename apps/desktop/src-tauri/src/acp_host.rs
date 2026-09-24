use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use harbor_acp::session::{AcpHostSession, ConfigOption, ResumeKind};
use harbor_acp::spawn::{SpawnSpec, plugin_mcp_servers};
use harbor_acp::transport::write_message;
use harbor_acp::{PermissionHook, permission_outcome};
use harbor_core::SqlitePool;
use harbor_core::types::{ContentPart, DetectedEngine};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};

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
    /// What the process was spawned with. A mismatch means reuse would serve
    /// the wrong engine, cwd, or roots — the session must be respawned.
    fingerprint: String,
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
    agent_id: Option<String>,
    /// Released plugin ids the agent may use; the sidecar still needs a stored
    /// credential before it advertises any tool.
    granted_plugins: Vec<String>,
    /// Engine options the builder chose earlier; re-applied on a fresh spawn.
    config: Vec<(String, Value)>,
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
    let path_env = harbor_core::engines::runtime_path();
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
        agent_id: None,
        // Folder threads belong to a workspace, not an agent — no plugin grants.
        granted_plugins: Vec::new(),
        config: ctx.config,
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
    let granted_plugins = harbor_core::plugins::list_grants(pool, &ctx.agent_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|grant| grant.enabled)
        .map(|grant| grant.plugin_id)
        .filter(|id| harbor_core::plugins::definition(id).is_some_and(|def| def.released))
        .collect();
    Ok(TurnContext {
        engine_id: ctx.engine_id,
        cwd,
        extra_roots,
        acp_session: ctx.acp_session,
        kind: SessionKind::Agent,
        agent_id: Some(ctx.agent_id),
        granted_plugins,
        config: ctx.config,
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
        if kind == SessionKind::Agent {
            let _ = block_on(harbor_core::chats::set_status(
                &pool,
                &session_ref,
                "needs_you",
            ));
        }
        let _ = app.emit(
            "acp_permission",
            permission_card(&perm_id, &session_ref, &params),
        );
        // The turn is now blocked until someone answers, possibly in a
        // workspace the builder is not looking at.
        block_on(crate::ipc::notify(
            &app,
            &pool,
            "permission",
            &format!(
                "{} needs permission",
                title.as_deref().unwrap_or("An agent")
            ),
            command
                .as_deref()
                .or(path.as_deref())
                .unwrap_or("Waiting for your answer"),
            harbor_core::notifications::Target::session(kind.as_str(), &session_ref),
        ));
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

/// Everything that decides which process a session is. A different engine,
/// working directory, or root set must respawn rather than reuse — the old
/// session's answers belong to the old context. Plugin grants are left out on
/// purpose: the sidecar re-checks the database per call, so toggling a grant
/// under Plugins must not kill a running session.
fn session_fingerprint(ctx: &TurnContext, command: &Path, args: &[String]) -> String {
    let mut roots = ctx.extra_roots.clone();
    roots.sort();
    json!({
        "engine": ctx.engine_id,
        "command": command.to_string_lossy(),
        "args": args,
        "cwd": ctx.cwd,
        "extraRoots": roots,
        "agent": ctx.agent_id,
    })
    .to_string()
}

fn get_or_connect(
    registry: &AcpRegistry,
    key: &str,
    ctx: &TurnContext,
    granted: &Path,
    args: Vec<String>,
    hook: PermissionHook,
) -> Result<Arc<LiveSession>, String> {
    let fingerprint = session_fingerprint(ctx, granted, &args);
    {
        let mut sessions = registry
            .sessions
            .lock()
            .map_err(|_| "acp registry".to_string())?;
        if let Some(live) = sessions.get(key) {
            if live.fingerprint == fingerprint {
                return Ok(live.clone());
            }
            // Context changed — the old process's remaining events must not
            // leak into the new session. Dropping it kills the child.
            sessions.remove(key);
        }
    }
    let spec = SpawnSpec {
        engine_id: ctx.engine_id.clone(),
        command: granted.display().to_string(),
        args,
        cwd: ctx.cwd.clone(),
        mcp_servers: std::env::current_exe()
            .map(|path| {
                plugin_mcp_servers(
                    &path.to_string_lossy(),
                    key,
                    ctx.agent_id.as_deref(),
                    &ctx.granted_plugins,
                    &crate::plugins_host::keyring_dir().display().to_string(),
                    &crate::database_path().display().to_string(),
                )
            })
            .unwrap_or_default(),
    };
    let mut session = AcpHostSession::connect(spec.clone()).map_err(|error| error.to_string())?;
    session.set_permission_hook(hook);
    let kind = session
        .open_session(ctx.acp_session.clone(), &spec, &ctx.extra_roots)
        .map_err(|error| error.to_string())?;
    // A respawn must not silently reset the options the builder chose — model,
    // mode, effort. An option the new session does not know is skipped, not
    // fatal.
    for (option_id, value) in &ctx.config {
        let _ = session.set_config_option(option_id, value.clone());
    }
    let _ = session.take_notifications();
    session.resume_kind = kind;
    let stdin = session.stdin_handle();
    let acp_id = session.session_id.clone();
    let live = Arc::new(LiveSession {
        session: Mutex::new(session),
        stdin,
        acp_id: Mutex::new(acp_id),
        fingerprint: fingerprint.clone(),
    });
    let mut sessions = registry
        .sessions
        .lock()
        .map_err(|_| "acp registry".to_string())?;
    if let Some(existing) = sessions.get(key) {
        if existing.fingerprint == fingerprint {
            return Ok(existing.clone());
        }
        // A concurrent spawn raced us for an older context — ours is newer.
        sessions.remove(key);
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
    if ctx.kind == SessionKind::Agent {
        let _ = harbor_core::chats::set_status(pool, session_ref, "running").await;
    }
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
        let turn = (|| {
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
                session.config_options.clone(),
            ))
        })();
        Ok::<_, String>((live, turn))
    })
    .await
    .map_err(|error| error.to_string())?;
    let (live, turn) = outcome?;
    let (result, notes, session_id, kind, config_options) = match turn {
        Ok(value) => value,
        Err(error) => {
            // Drop only the session that actually failed — a stale turn must
            // not kill the replacement that now owns this key.
            let _ = registry_for_err.sessions.lock().map(|mut sessions| {
                if sessions
                    .get(session_ref)
                    .is_some_and(|current| Arc::ptr_eq(current, &live))
                {
                    sessions.remove(session_ref);
                }
            });
            if error.contains("auth-required") {
                let _ = app.emit(
                    "engine_auth_required",
                    json!({ "engineId": engine_id, "hint": "CLI login" }),
                );
            }
            if persist_kind == SessionKind::Agent {
                let _ = harbor_core::chats::set_status(pool, session_ref, "error").await;
            }
            return Err(error);
        }
    };
    // The session was swapped mid-turn (engine switch, new roots, respawn):
    // this reply belongs to a process that is already gone — discard it.
    let still_current = registry_for_err
        .sessions
        .lock()
        .map(|sessions| {
            sessions
                .get(session_ref)
                .is_some_and(|current| Arc::ptr_eq(current, &live))
        })
        .unwrap_or(false);
    if !still_current {
        if persist_kind == SessionKind::Agent {
            let _ = harbor_core::chats::set_status(pool, session_ref, "idle").await;
        }
        return Ok(());
    }
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
    // The turn is over. Say so, and say why it ended if the engine gave up —
    // unless the builder watched it happen.
    let watched = app
        .try_state::<crate::ipc::Watching>()
        .is_some_and(|watching| crate::ipc::is_being_watched(app, &watching, session_ref));
    // A reply the builder was not watching marks the folder thread unread —
    // the same "did you see it" question the inbox row asks.
    if chat_kind == "thread" && !prose.is_empty() && !watched {
        let _ = harbor_core::threads::mark_unread(pool, session_ref).await;
    }
    let stop = result
        .get("stopReason")
        .and_then(Value::as_str)
        .unwrap_or("end_turn");
    let (kind, title, body) = match stop {
        "end_turn" => (
            "turn-finished",
            "Finished",
            first_line(&prose).unwrap_or_else(|| "The agent finished its turn.".into()),
        ),
        "cancelled" => (
            "turn-cancelled",
            "Cancelled",
            "The turn was cancelled.".into(),
        ),
        "refusal" => (
            "turn-refused",
            "Refused",
            "The engine refused the turn.".into(),
        ),
        other => (
            "turn-stopped",
            "Stopped",
            format!("The turn stopped: {other}."),
        ),
    };
    if !watched {
        crate::ipc::notify(
            app,
            pool,
            kind,
            title,
            &body,
            harbor_core::notifications::Target::session(chat_kind, session_ref),
        )
        .await;
    }
    if persist_kind == SessionKind::Agent {
        let _ = harbor_core::chats::set_status(pool, session_ref, "idle").await;
    }
    Ok(())
}

/// The opening line of a reply is what makes a finished-turn row worth reading.
fn first_line(prose: &str) -> Option<String> {
    let line = prose.lines().map(str::trim).find(|line| !line.is_empty())?;
    let trimmed: String = line.chars().take(120).collect();
    Some(if trimmed.len() < line.len() {
        format!("{trimmed}…")
    } else {
        trimmed
    })
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
    let agent_id = ctx
        .agent_id
        .clone()
        .ok_or_else(|| "agent chat is missing its teammate".to_string())?;
    let briefing = harbor_core::briefing::load(pool, &agent_id)
        .await
        .map_err(|error| error.to_string())?;
    let parts = harbor_core::briefing::attach(&briefing, parts);
    run_turn(app, pool, allow, registry, chat_id, ctx, &parts).await
}

/// Connect (or reuse) the session just to read what the agent offers, so the
/// picker can list real models before the first message is ever sent.
async fn config_options(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    session_ref: &str,
    ctx: TurnContext,
) -> Result<Vec<ConfigOption>, String> {
    let (command, args) = acp_command(&ctx.engine_id)?;
    let granted = allow
        .grant(&command, ExecutableKind::Engine)
        .and_then(|path| allow.authorize(&path, ExecutableKind::Engine))
        .map_err(|error| error.to_string())?;
    let registry = registry.clone();
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
    tauri::async_runtime::spawn_blocking(move || {
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
        let session = live.session.lock().map_err(|_| "acp session".to_string())?;
        Ok::<_, String>(session.config_options.clone())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub async fn thread_config_options(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    thread_id: &str,
) -> Result<Vec<ConfigOption>, String> {
    let ctx = thread_turn_context(pool, thread_id).await?;
    config_options(app, pool, allow, registry, thread_id, ctx).await
}

pub async fn agent_config_options(
    app: &AppHandle,
    pool: &SqlitePool,
    allow: &ExecutableAllowlist,
    registry: &AcpRegistry,
    chat_id: &str,
) -> Result<Vec<ConfigOption>, String> {
    let ctx = agent_turn_context(pool, chat_id).await?;
    config_options(app, pool, allow, registry, chat_id, ctx).await
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

/// A restart kills every engine process but leaves their database state
/// behind. Rows stuck mid-turn would otherwise look alive forever: chats go
/// back to idle and unanswered permission requests are cancelled, so a fresh
/// session never inherits a dead one's state. Stored `acp_session` ids are
/// kept — the engine may still be able to resume them; if not, open falls
/// back to a new session.
pub async fn reconcile_stale_sessions(pool: &SqlitePool) {
    let _ = sqlx::query(
        "UPDATE agent_chats SET status = 'idle' WHERE status IN ('running', 'needs_you')",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query("UPDATE acp_permissions SET status = 'cancelled' WHERE status = 'pending'")
        .execute(pool)
        .await;
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
    use super::first_line;

    #[test]
    fn finished_turn_body_uses_the_first_real_line_and_caps_it() {
        assert_eq!(first_line("").as_deref(), None);
        assert_eq!(first_line("   \n\n  ").as_deref(), None);
        assert_eq!(
            first_line("\n\n  Ran the tests.  \nAll green.").as_deref(),
            Some("Ran the tests.")
        );
        let long = "x".repeat(200);
        let capped = first_line(&long).unwrap();
        assert_eq!(capped.chars().count(), 121, "120 chars plus an ellipsis");
        assert!(capped.ends_with('…'));
    }

    use super::*;
    use harbor_core::types::CreateAgent;

    fn looks_tokenish(value: &str) -> bool {
        harbor_plugins::keyring::looks_like_github_user_token(value)
            || value.starts_with("ghp_")
            || value.starts_with("github_pat_")
            || value.starts_with("sk-")
            || value.contains("Bearer ")
    }

    #[test]
    fn live_spawn_spec_includes_harbor_plugins_mcp_without_tokens() {
        let granted = vec!["github".to_string()];
        let servers =
            plugin_mcp_servers("/usr/bin/harbor", "thread-abc", None, &granted, "/k", "/db");
        assert_eq!(servers.len(), 1);
        let server = &servers[0];
        assert_eq!(server.name, "harbor-plugins");
        assert_eq!(server.command, "/usr/bin/harbor");
        assert_eq!(server.args, ["mcp-plugins", "--session", "thread-abc"]);
        let encoded = serde_json::to_value(server).unwrap();
        assert!(encoded["env"].is_array());
        assert!(!encoded["env"].is_object());
        assert_eq!(encoded["env"].as_array().unwrap().len(), 3);
        assert_eq!(encoded["env"][0]["name"], "HARBOR_PLUGIN_SESSION");
        assert_eq!(encoded["env"][0]["value"], "thread-abc");
        assert_eq!(encoded["env"][1]["name"], "HARBOR_PLUGIN_GRANTS");
        assert_eq!(encoded["env"][1]["value"], "github");
        for env in &server.env {
            assert!(!env.name.to_ascii_uppercase().contains("TOKEN"));
            assert!(!looks_tokenish(&env.value), "{}", env.value);
        }
        assert!(!looks_tokenish(&encoded.to_string()));
        assert!(plugin_mcp_servers("", "thread-abc", None, &granted, "/k", "/db").is_empty());
    }

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
                face_index: None,
                home_path: None,
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
        assert_eq!(ctx.agent_id.as_deref(), Some(agent.id.as_str()));

        harbor_core::memory::upsert(&pool, &agent.id, "Prefers tests")
            .await
            .unwrap();
        let briefing = harbor_core::briefing::load(&pool, &agent.id).await.unwrap();
        let parts = harbor_core::briefing::attach(
            &briefing,
            &[ContentPart {
                r#type: "text".into(),
                text: Some("hello agent".into()),
                path: None,
            }],
        );
        assert!(
            parts[0]
                .text
                .as_deref()
                .unwrap()
                .contains("<harbor-agent-data>")
        );
        assert_eq!(parts[1].text.as_deref(), Some("hello agent"));
        let history = harbor_core::chats::history(&pool, &chat.id).await.unwrap();
        assert!(
            history
                .iter()
                .all(|line| !line.text.contains("<harbor-agent-data>"))
        );
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

    fn test_ctx() -> super::TurnContext {
        super::TurnContext {
            engine_id: "opencode".into(),
            cwd: "/tmp/work".into(),
            extra_roots: vec!["/tmp/extra".into()],
            acp_session: None,
            kind: super::SessionKind::Thread,
            agent_id: None,
            granted_plugins: vec!["github".into()],
            config: vec![],
        }
    }

    #[test]
    fn fingerprint_changes_with_spawn_context_but_not_grants() {
        use std::path::Path;
        let ctx = test_ctx();
        let base = super::session_fingerprint(&ctx, Path::new("/usr/bin/opencode"), &[]);
        assert_eq!(
            base,
            super::session_fingerprint(&ctx, Path::new("/usr/bin/opencode"), &[])
        );

        let other_engine = super::TurnContext {
            engine_id: "claude-code".into(),
            ..test_ctx()
        };
        assert_ne!(
            base,
            super::session_fingerprint(&other_engine, Path::new("/usr/bin/opencode"), &[])
        );

        let other_cwd = super::TurnContext {
            cwd: "/tmp/other".into(),
            ..test_ctx()
        };
        assert_ne!(
            base,
            super::session_fingerprint(&other_cwd, Path::new("/usr/bin/opencode"), &[])
        );

        let other_roots = super::TurnContext {
            extra_roots: vec!["/tmp/a".into(), "/tmp/b".into()],
            ..test_ctx()
        };
        assert_ne!(
            base,
            super::session_fingerprint(&other_roots, Path::new("/usr/bin/opencode"), &[])
        );

        // Grant toggles ride the live DB check — they must not force a respawn.
        let other_grants = super::TurnContext {
            granted_plugins: vec![],
            ..test_ctx()
        };
        assert_eq!(
            base,
            super::session_fingerprint(&other_grants, Path::new("/usr/bin/opencode"), &[])
        );
    }

    #[tokio::test]
    async fn restart_reconciles_stale_chats_and_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let pool = harbor_core::db::open(&dir.path().join("db.sqlite"))
            .await
            .unwrap();
        let agent = harbor_core::agents::create(
            &pool,
            CreateAgent {
                name: "Mate".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                home_path: None,
                face_index: None,
            },
        )
        .await
        .unwrap();
        let chat = harbor_core::chats::create(&pool, &agent.id).await.unwrap();
        harbor_core::chats::set_status(&pool, &chat.id, "running")
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO acp_permissions
             (id, session_ref, session_kind, tool_title, path, command, options_json, status, created_at)
             VALUES ('stale-perm', ?1, 'agent', 'Run', NULL, NULL, '[]', 'pending', 1)",
        )
        .bind(&chat.id)
        .execute(&pool)
        .await
        .unwrap();

        super::reconcile_stale_sessions(&pool).await;

        let (status,): (String,) = sqlx::query_as("SELECT status FROM agent_chats WHERE id = ?1")
            .bind(&chat.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "idle");
        let (perm,): (String,) =
            sqlx::query_as("SELECT status FROM acp_permissions WHERE id = 'stale-perm'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(perm, "cancelled");
    }
}
