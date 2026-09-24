use crate::acp_host::AcpRegistry;
use crate::security::ExecutableAllowlist;
use harbor_core::SqlitePool;
use harbor_core::icons::EngineIcon;
use harbor_core::types::{
    AgentChat, AgentRecord, ChatMessage, ContentPart, CreateAgent, DetectedEngine, FileDiff,
    FsEntry, Memory, Notification, PaneLayout, PaneState, Place, PluginApproval, PluginGrant,
    PluginRow, SearchHit, ThreadRecord, UpdateAgent, UpdateStatus, Workspace, WorkspaceSetup,
    WorkspaceTab,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;

fn map_err(error: harbor_core::error::Error) -> String {
    error.to_string()
}

/// The native folder picker is the only door into `fs:scope`. It stores the
/// picked path host-side and hands the renderer an opaque, single-use pick id;
/// commands that create filesystem access consume the id instead of trusting a
/// renderer-supplied path.
#[derive(Default)]
pub struct PendingPicks(Mutex<HashMap<String, (String, Instant)>>);

/// Long enough to finish the dialog flow that follows the pick, short enough
/// that a leaked id cannot be redeemed far in the future.
const PICK_TTL: Duration = Duration::from_secs(10 * 60);

impl PendingPicks {
    fn insert(&self, path: &str) -> String {
        let mut picks = self.0.lock().unwrap();
        picks.retain(|_, (_, at)| at.elapsed() < PICK_TTL);
        let id = uuid::Uuid::now_v7().simple().to_string();
        picks.insert(id.clone(), (path.to_string(), Instant::now()));
        id
    }

    fn take(&self, id: &str) -> Result<String, String> {
        match self.0.lock().unwrap().remove(id) {
            Some((path, at)) if at.elapsed() < PICK_TTL => Ok(path),
            _ => Err("that folder selection expired — pick it again".into()),
        }
    }
}

/// What the renderer sees after a native pick: a display path plus the
/// single-use capability that lets exactly one follow-up command use it.
#[derive(serde::Serialize)]
pub struct FolderPick {
    id: String,
    path: String,
}

fn allow_workspace_directory(app: &AppHandle, folder: &str) {
    let _ = app.fs_scope().allow_directory(Path::new(folder), true);
}

/// Drop `folder` from the fs scope once nothing in the database references it
/// anymore — workspaces, places, and agent home folders share the scope.
async fn revoke_folder_scope(app: &AppHandle, pool: &SqlitePool, folder: &str) {
    let referenced: bool = sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM workspaces WHERE folder = ?1
             UNION ALL SELECT 1 FROM places WHERE path = ?1
             UNION ALL SELECT 1 FROM agents WHERE home_path = ?1
         )",
    )
    .bind(folder)
    .fetch_one(pool)
    .await
    .unwrap_or(true);
    if !referenced {
        let _ = app.fs_scope().forbid_directory(Path::new(folder), true);
    }
}

fn fetch_url(url: &str) -> Result<String, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .and_then(|client| {
            client
                .get(url)
                .header("User-Agent", "harbor")
                .send()
                .and_then(|response| response.text())
        })
        .map_err(|error| error.to_string())
}

/// A signed installer should be tens of megabytes, not a stream that can
/// exhaust memory before verification ever runs.
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;

/// Artifacts are binary; a longer budget than the metadata fetch. Streams
/// through `out` so a multi-hundred-megabyte release never sits in memory —
/// the cap applies to bytes actually written.
fn fetch_stream(url: &str, out: &mut dyn std::io::Write) -> Result<(), String> {
    use std::io::Read;
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .and_then(|client| {
            client
                .get(url)
                .header("User-Agent", "harbor")
                .send()
                .and_then(|response| response.error_for_status())
        })
        .map_err(|error| error.to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ARTIFACT_BYTES)
    {
        return Err("update artifact is too large".into());
    }
    let copied = std::io::copy(&mut response.take(MAX_ARTIFACT_BYTES + 1), out)
        .map_err(|error| error.to_string())?;
    if copied > MAX_ARTIFACT_BYTES {
        return Err("update artifact is too large".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn settings_get(pool: State<'_, SqlitePool>, key: String) -> Result<Value, String> {
    harbor_core::commands::settings_get(&pool, &key)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn settings_set(
    pool: State<'_, SqlitePool>,
    key: String,
    value: Value,
) -> Result<(), String> {
    harbor_core::commands::settings_set(&pool, &key, value)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn engines_detect(
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
) -> Result<Vec<DetectedEngine>, String> {
    let engines = harbor_core::commands::engines_detect(&pool)
        .await
        .map_err(map_err)?;
    crate::acp_host::grant_engines(&allow, &engines);
    Ok(engines)
}

#[tauri::command]
pub async fn engines_recheck(
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
) -> Result<Vec<DetectedEngine>, String> {
    let engines = harbor_core::commands::engines_recheck(&pool)
        .await
        .map_err(map_err)?;
    crate::acp_host::grant_engines(&allow, &engines);
    Ok(engines)
}

/// Fetch the ACP adapter an engine needs, then report the fresh engine list so
/// the caller sees the engine become usable in the same round trip.
#[tauri::command]
pub async fn engine_install_adapter(
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    engine_id: String,
) -> Result<Vec<DetectedEngine>, String> {
    tauri::async_runtime::spawn_blocking(move || harbor_core::engines::install_adapter(&engine_id))
        .await
        .map_err(|error| error.to_string())??;
    let engines = harbor_core::commands::engines_recheck(&pool)
        .await
        .map_err(map_err)?;
    crate::acp_host::grant_engines(&allow, &engines);
    Ok(engines)
}

#[tauri::command]
pub async fn workspace_list(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<Workspace>, String> {
    let workspaces = harbor_core::commands::workspace_list(&pool)
        .await
        .map_err(map_err)?;
    for workspace in &workspaces {
        allow_workspace_directory(&app, &workspace.folder);
    }
    Ok(workspaces)
}

/// The native picker stores the chosen folder host-side and returns a
/// single-use pick id. No filesystem scope is granted until a command consumes
/// the pick and persists a row that needs the folder.
#[tauri::command]
pub async fn workspace_pick_folder(
    app: AppHandle,
    picks: State<'_, PendingPicks>,
) -> Result<Option<FolderPick>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a workspace folder")
            .blocking_pick_folder()
            .and_then(|path| path.into_path().ok())
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| error.to_string())?;
    Ok(picked.map(|path| FolderPick {
        id: picks.insert(&path),
        path,
    }))
}

#[tauri::command]
pub async fn workspace_add(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    picks: State<'_, PendingPicks>,
    pick_id: String,
) -> Result<Workspace, String> {
    let folder = picks.take(&pick_id)?;
    let workspace = harbor_core::commands::workspace_add(&pool, folder)
        .await
        .map_err(map_err)?;
    allow_workspace_directory(&app, &workspace.folder);
    Ok(workspace)
}

#[tauri::command]
pub async fn workspace_remove(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    let folder: Option<String> = sqlx::query_scalar("SELECT folder FROM workspaces WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| error.to_string())?;
    harbor_core::commands::workspace_remove(&pool, id)
        .await
        .map_err(map_err)?;
    if let Some(folder) = folder {
        revoke_folder_scope(&app, &pool, &folder).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn workspace_pin(
    pool: State<'_, SqlitePool>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    harbor_core::commands::workspace_pin(&pool, id, pinned)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_rename(
    pool: State<'_, SqlitePool>,
    id: String,
    title: String,
) -> Result<Workspace, String> {
    harbor_core::commands::workspace_rename(&pool, id, title)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_save_layout(
    pool: State<'_, SqlitePool>,
    tab_id: String,
    layout: PaneLayout,
) -> Result<(), String> {
    harbor_core::commands::workspace_save_layout(&pool, tab_id, layout)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_tidy(
    pool: State<'_, SqlitePool>,
    tab_id: String,
) -> Result<PaneLayout, String> {
    harbor_core::commands::workspace_tidy(&pool, tab_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_ensure_tab(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
) -> Result<WorkspaceTab, String> {
    harbor_core::commands::workspace_ensure_tab(&pool, workspace_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_configure_tab(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    setup: WorkspaceSetup,
) -> Result<WorkspaceTab, String> {
    harbor_core::commands::workspace_configure_tab(&pool, workspace_id, setup)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn layout_restore(pool: State<'_, SqlitePool>) -> Result<Vec<WorkspaceTab>, String> {
    harbor_core::commands::layout_restore(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pane_create(
    pool: State<'_, SqlitePool>,
    tab_id: String,
    kind: String,
    state: PaneState,
) -> Result<String, String> {
    harbor_core::commands::pane_create(&pool, tab_id, kind, state)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pane_close(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::pane_close(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pane_set_engine(
    pool: State<'_, SqlitePool>,
    id: String,
    engine_id: String,
) -> Result<(), String> {
    harbor_core::commands::pane_set_engine(&pool, id, engine_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn fs_read(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    path: String,
) -> Result<String, String> {
    harbor_core::commands::fs_read(&pool, workspace_id, path)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn fs_write(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    path: String,
    contents: String,
) -> Result<(), String> {
    harbor_core::commands::fs_write(&pool, workspace_id, path, contents)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn fs_list(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
    path: String,
) -> Result<Vec<FsEntry>, String> {
    harbor_core::commands::fs_list(&pool, workspace_id, path)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_list(
    pool: State<'_, SqlitePool>,
    workspace_id: Option<String>,
) -> Result<Vec<ThreadRecord>, String> {
    harbor_core::commands::thread_list(&pool, workspace_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_list_all(pool: State<'_, SqlitePool>) -> Result<Vec<ThreadRecord>, String> {
    harbor_core::commands::thread_list_all(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_mark_read(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::thread_mark_read(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_history(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<Vec<ChatMessage>, String> {
    harbor_core::threads::history(&pool, &id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_create(
    pool: State<'_, SqlitePool>,
    workspace_id: Option<String>,
    engine_id: String,
) -> Result<ThreadRecord, String> {
    harbor_core::commands::thread_create(&pool, workspace_id, engine_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_rename(
    pool: State<'_, SqlitePool>,
    id: String,
    title: String,
) -> Result<(), String> {
    harbor_core::commands::thread_rename(&pool, id, title)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_delete(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
) -> Result<(), String> {
    crate::acp_host::drop_session(&registry, &id);
    harbor_core::commands::thread_delete(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_pin(
    pool: State<'_, SqlitePool>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    harbor_core::commands::thread_pin(&pool, id, pinned)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_send(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::thread_send(&pool, id.clone(), parts.clone())
        .await
        .map_err(map_err)?;
    crate::acp_host::prompt(&app, &pool, &allow, &registry, &id, &parts).await
}

#[tauri::command]
pub async fn thread_cancel(registry: State<'_, AcpRegistry>, id: String) -> Result<(), String> {
    crate::acp_host::cancel(&registry, &id).await
}

#[tauri::command]
pub async fn thread_set_config(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
    option_id: String,
    value: Value,
) -> Result<(), String> {
    harbor_core::commands::thread_set_config(&pool, id.clone(), option_id.clone(), value.clone())
        .await
        .map_err(map_err)?;
    crate::acp_host::set_live_config(&registry, &id, option_id, value).await
}

/// What this thread's engine offers right now. Connects if it is not running:
/// the agent only lists its models once a session exists.
#[tauri::command]
pub async fn thread_config_options(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    id: String,
) -> Result<Vec<harbor_acp::session::ConfigOption>, String> {
    crate::acp_host::thread_config_options(&app, &pool, &allow, &registry, &id).await
}

/// The live session speaks the old engine's protocol state, so it goes first.
#[tauri::command]
pub async fn thread_set_engine(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
    engine_id: String,
) -> Result<(), String> {
    crate::acp_host::drop_session(&registry, &id);
    harbor_core::commands::thread_set_engine(&pool, id, engine_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_attach_files(
    pool: State<'_, SqlitePool>,
    id: String,
    paths: Vec<String>,
) -> Result<(), String> {
    harbor_core::commands::thread_attach_files(&pool, id, paths)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_list(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AgentRecord>, String> {
    let agents = harbor_core::commands::agent_list(&pool)
        .await
        .map_err(map_err)?;
    if let Ok(paths) = harbor_core::places::granted_paths(&pool).await {
        for path in paths {
            allow_workspace_directory(&app, &path);
        }
    }
    Ok(agents)
}

#[tauri::command]
pub async fn agent_create(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    picks: State<'_, PendingPicks>,
    input: CreateAgent,
    home_pick_id: Option<String>,
) -> Result<AgentRecord, String> {
    let mut input = input;
    // A home folder is a filesystem grant, so it must come from the picker.
    match home_pick_id {
        Some(pick_id) => {
            input.home_path = Some(
                harbor_core::places::canonicalize_folder(&picks.take(&pick_id)?)
                    .map_err(map_err)?,
            );
        }
        None => {
            if input
                .home_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty())
            {
                return Err("the home folder must be chosen in the folder picker".into());
            }
        }
    }
    let home = input.home_path.clone();
    let agent = harbor_core::commands::agent_create(&pool, input)
        .await
        .map_err(map_err)?;
    if let Some(path) = home.filter(|path| !path.trim().is_empty()) {
        allow_workspace_directory(&app, &path);
    }
    Ok(agent)
}

#[tauri::command]
pub async fn agent_update(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    picks: State<'_, PendingPicks>,
    input: UpdateAgent,
    home_pick_id: Option<String>,
) -> Result<(), String> {
    let mut input = input;
    let existing_home: Option<String> =
        sqlx::query_scalar("SELECT home_path FROM agents WHERE id = ?1")
            .bind(&input.id)
            .fetch_optional(&*pool)
            .await
            .map_err(|error| error.to_string())?
            .flatten();
    match home_pick_id {
        Some(pick_id) => {
            input.home_path = Some(
                harbor_core::places::canonicalize_folder(&picks.take(&pick_id)?)
                    .map_err(map_err)?,
            );
        }
        None => {
            // Without a pick the renderer may only keep the stored value or
            // clear it — a different path would be a fabricated grant.
            if let Some(path) = &input.home_path {
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    input.home_path = Some(String::new());
                } else if existing_home.as_deref() != Some(trimmed) {
                    return Err("the home folder must be chosen in the folder picker".into());
                }
            }
        }
    }
    let home = input.home_path.clone();
    harbor_core::commands::agent_update(&pool, input)
        .await
        .map_err(map_err)?;
    // `home == None` keeps the stored value; `Some("")` clears it.
    let effective = match &home {
        Some(path) if !path.trim().is_empty() => Some(path.clone()),
        Some(_) => None,
        None => existing_home.clone(),
    };
    if let Some(path) = &effective {
        allow_workspace_directory(&app, path);
    }
    let previous = existing_home.filter(|path| !path.trim().is_empty());
    if let Some(previous) = previous.filter(|old| effective.as_ref() != Some(old)) {
        revoke_folder_scope(&app, &pool, &previous).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_delete(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
) -> Result<(), String> {
    let home: Option<String> = sqlx::query_scalar("SELECT home_path FROM agents WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| error.to_string())?
        .flatten();
    crate::acp_host::drop_agent_sessions(&pool, &registry, &id).await;
    harbor_core::commands::agent_delete(&pool, id)
        .await
        .map_err(map_err)?;
    if let Some(home) = home.filter(|path| !path.trim().is_empty()) {
        revoke_folder_scope(&app, &pool, &home).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_draft_with_ai(
    pool: State<'_, SqlitePool>,
    hint: String,
) -> Result<CreateAgent, String> {
    harbor_core::commands::agent_draft_with_ai(&pool, hint)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_list(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<Vec<AgentChat>, String> {
    harbor_core::commands::agent_chat_list(&pool, agent_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_create(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<AgentChat, String> {
    harbor_core::commands::agent_chat_create(&pool, agent_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_history(
    pool: State<'_, SqlitePool>,
    chat_id: String,
) -> Result<Vec<ChatMessage>, String> {
    harbor_core::commands::agent_chat_history(&pool, chat_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_send(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_send(&pool, chat_id.clone(), parts.clone())
        .await
        .map_err(map_err)?;
    crate::acp_host::prompt_agent(&app, &pool, &allow, &registry, &chat_id, &parts).await
}

#[tauri::command]
pub async fn agent_chat_cancel(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
) -> Result<(), String> {
    crate::acp_host::cancel(&registry, &chat_id).await?;
    harbor_core::commands::agent_chat_cancel(&pool, chat_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_rename(
    pool: State<'_, SqlitePool>,
    chat_id: String,
    title: String,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_rename(&pool, chat_id, title)
        .await
        .map_err(map_err)
}

/// The live session dies with the chat it belonged to.
#[tauri::command]
pub async fn agent_chat_delete(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
) -> Result<(), String> {
    crate::acp_host::drop_session(&registry, &chat_id);
    harbor_core::commands::agent_chat_delete(&pool, chat_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_config_options(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
) -> Result<Vec<harbor_acp::session::ConfigOption>, String> {
    crate::acp_host::agent_config_options(&app, &pool, &allow, &registry, &chat_id).await
}

#[tauri::command]
pub async fn agent_chat_set_config(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
    option_id: String,
    value: Value,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_set_config(
        &pool,
        chat_id.clone(),
        option_id.clone(),
        value.clone(),
    )
    .await
    .map_err(map_err)?;
    crate::acp_host::set_live_config(&registry, &chat_id, option_id, value).await
}

#[tauri::command]
pub async fn memory_list(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<Vec<Memory>, String> {
    harbor_core::commands::memory_list(&pool, agent_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn memory_upsert(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    body: String,
) -> Result<Memory, String> {
    harbor_core::commands::memory_upsert(&pool, agent_id, body)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn memory_delete(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::memory_delete(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn places_list(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<Vec<Place>, String> {
    harbor_core::commands::places_list(&pool, agent_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn places_grant(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    picks: State<'_, PendingPicks>,
    agent_id: String,
    pick_id: String,
) -> Result<(), String> {
    // Canonicalize before validating so the scope below covers the exact
    // canonical path the row stores — e.g. macOS resolves /tmp to /private/tmp.
    let path = harbor_core::places::canonicalize_folder(&picks.take(&pick_id)?).map_err(map_err)?;
    harbor_core::commands::places_grant(&pool, agent_id, path.clone())
        .await
        .map_err(map_err)?;
    allow_workspace_directory(&app, &path);
    Ok(())
}

#[tauri::command]
pub async fn places_revoke(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    let path: Option<String> = sqlx::query_scalar("SELECT path FROM places WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| error.to_string())?;
    harbor_core::commands::places_revoke(&pool, id)
        .await
        .map_err(map_err)?;
    if let Some(path) = path {
        revoke_folder_scope(&app, &pool, &path).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn session_search(
    pool: State<'_, SqlitePool>,
    agent_id: Option<String>,
    workspace_id: Option<String>,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    harbor_core::commands::session_search(&pool, agent_id, workspace_id, query)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn mail_send(
    pool: State<'_, SqlitePool>,
    from_agent_id: String,
    to_agent_id: String,
    body: String,
) -> Result<(), String> {
    harbor_core::commands::mail_send(&pool, from_agent_id, to_agent_id, body)
        .await
        .map_err(map_err)
}

/// Where a builder may drop real engine logos. Harbor ships original marks and
/// does not redistribute vendor trademarks, so this directory starts empty.
pub struct EngineIconDir(pub std::path::PathBuf);

#[tauri::command]
pub fn engine_icons(dir: State<'_, EngineIconDir>) -> Result<Vec<EngineIcon>, String> {
    harbor_core::icons::engine_icons(&dir.0).map_err(map_err)
}

#[tauri::command]
pub fn default_profile_name() -> String {
    harbor_core::commands::default_profile_name()
}

/// Live bell events default on. Only an explicit JSON `false` silences them;
/// the inbox row is still recorded either way.
fn should_emit_notification(enabled: Option<&Value>) -> bool {
    !matches!(enabled, Some(Value::Bool(false)))
}

/// Records an event and tells the renderer, so the bell updates without polling.
///
/// `notification_kinds` is an optional JSON array of enabled kind ids. A missing
/// setting enables every kind; a kind outside the list is neither recorded nor
/// emitted.
///
/// `notifications_enabled` defaults on. An explicit `false` still writes the
/// inbox row (history on next open) but skips the live event so the badge
/// stays quiet.
pub async fn notify(
    app: &AppHandle,
    pool: &SqlitePool,
    kind: &str,
    title: &str,
    body: &str,
    target: harbor_core::notifications::Target,
) {
    let kinds = harbor_core::settings::get(pool, "notification_kinds")
        .await
        .ok()
        .flatten();
    if !harbor_core::notifications::kind_allowed(kinds, kind) {
        return;
    }
    if let Ok(row) = harbor_core::notifications::record(pool, kind, title, body, target).await {
        let enabled = harbor_core::settings::get(pool, "notifications_enabled")
            .await
            .ok()
            .flatten();
        if should_emit_notification(enabled.as_ref()) {
            let _ = app.emit("notification", row);
            // An OS toast only helps when the reply is not already on screen —
            // while the window has focus the in-app event is the signal.
            let focused = app
                .get_webview_window("main")
                .and_then(|window| window.is_focused().ok())
                .unwrap_or(false);
            if !focused {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().builder().title(title).body(body).show();
            }
        }
    }
}

/// The conversation the builder currently has open, if any.
#[derive(Default)]
pub struct Watching(pub std::sync::Mutex<Option<String>>);

#[tauri::command]
pub fn session_watch(watching: State<'_, Watching>, session_ref: Option<String>) {
    if let Ok(mut current) = watching.0.lock() {
        *current = session_ref;
    }
}

/// An end-of-turn row is for work you were not there to see. Reading the reply
/// as it lands is already the notification, so skip the row in that one case:
/// this conversation is open and the window has focus.
pub fn is_being_watched(app: &AppHandle, watching: &Watching, session_ref: &str) -> bool {
    let open = watching
        .0
        .lock()
        .ok()
        .and_then(|current| current.clone())
        .is_some_and(|current| current == session_ref);
    open && app
        .get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn notifications_unread_count(pool: State<'_, SqlitePool>) -> Result<i64, String> {
    harbor_core::commands::notifications_unread_count(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn notifications_list(pool: State<'_, SqlitePool>) -> Result<Vec<Notification>, String> {
    harbor_core::commands::notifications_list(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn notifications_mark_read(pool: State<'_, SqlitePool>) -> Result<(), String> {
    harbor_core::commands::notifications_mark_read(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn notifications_clear(pool: State<'_, SqlitePool>) -> Result<(), String> {
    harbor_core::commands::notifications_clear(&pool)
        .await
        .map_err(map_err)
}

/// Local surfaces that schedule their own work (routines) record inbox events
/// through the same path as host-raised ones: `notification_kinds` filters
/// them, the live emit respects `notifications_enabled`, and a bad row is an
/// error the caller sees instead of a silent drop.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn notification_record(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    kind: String,
    title: String,
    body: Option<String>,
    mode: Option<String>,
    session_ref: Option<String>,
    workspace_id: Option<String>,
    pane_id: Option<String>,
) -> Result<(), String> {
    if !harbor_core::notifications::KNOWN_KINDS.contains(&kind.as_str()) {
        return Err(format!("unknown notification kind: {kind}"));
    }
    let kinds = harbor_core::settings::get(&pool, "notification_kinds")
        .await
        .ok()
        .flatten();
    if !harbor_core::notifications::kind_allowed(kinds, &kind) {
        return Ok(());
    }
    let row = harbor_core::notifications::record(
        &pool,
        &kind,
        &title,
        body.as_deref().unwrap_or(""),
        harbor_core::notifications::Target {
            mode,
            workspace_id,
            pane_id,
            session_ref,
        },
    )
    .await
    .map_err(map_err)?;
    let enabled = harbor_core::settings::get(&pool, "notifications_enabled")
        .await
        .ok()
        .flatten();
    if should_emit_notification(enabled.as_ref()) {
        let _ = app.emit("notification", row);
    }
    Ok(())
}

#[tauri::command]
pub async fn face_preview(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    face_index: i32,
) -> Result<String, String> {
    harbor_core::commands::face_preview(&pool, agent_id, face_index)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn acp_permission_resolve(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
    option_id: Option<String>,
    cancelled: bool,
) -> Result<(), String> {
    crate::acp_host::resolve_permission(&pool, &registry, &id, option_id, cancelled).await
}

#[tauri::command]
pub async fn plugin_list(pool: State<'_, SqlitePool>) -> Result<Vec<PluginRow>, String> {
    harbor_core::commands::plugin_list(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_connect(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    crate::plugins_host::connect(app, pool.inner().clone(), id).await
}

#[tauri::command]
pub async fn plugin_configure(
    pool: State<'_, SqlitePool>,
    id: String,
    credential: String,
    account_label: Option<String>,
) -> Result<(), String> {
    crate::plugins_host::configure(&pool, id, credential, account_label).await
}

#[tauri::command]
pub async fn plugin_disconnect(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    crate::plugins_host::disconnect(&pool, id).await
}

#[tauri::command]
pub async fn plugin_set_agent_grant(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    crate::plugins_host::set_agent_grant(&pool, agent_id, plugin_id, enabled).await
}

#[tauri::command]
pub async fn plugin_resolve_approval(
    pool: State<'_, SqlitePool>,
    id: String,
    allow: bool,
) -> Result<(), String> {
    crate::plugins_host::resolve_approval(&pool, id, allow).await
}

#[tauri::command]
pub async fn plugin_approvals_list(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<PluginApproval>, String> {
    harbor_core::commands::plugin_approvals_list(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_grants_list(
    pool: State<'_, SqlitePool>,
    agent_id: String,
) -> Result<Vec<PluginGrant>, String> {
    harbor_core::commands::plugin_grants_list(&pool, agent_id)
        .await
        .map_err(map_err)
}

fn dictation_payload(event: harbor_speech::DictationEvent) -> Value {
    json!({
        "state": event.state,
        "copy": event.copy
    })
}

#[tauri::command]
pub async fn dictation_begin() -> Result<(), String> {
    Err("unimplemented: dictation_begin".into())
}

#[tauri::command]
pub async fn dictation_end() -> Result<(), String> {
    Err("unimplemented: dictation_end".into())
}

#[tauri::command]
pub async fn dictation_devices() -> Result<Vec<Value>, String> {
    Ok(harbor_speech::list_devices()
        .into_iter()
        .map(|device| {
            json!({
                "id": device.id,
                "label": device.label,
                "selected": device.selected
            })
        })
        .collect())
}

#[tauri::command]
pub async fn dictation_prepare_model(app: AppHandle) -> Result<(), String> {
    let event = harbor_speech::model_missing();
    let _ = app.emit("dictation_state", dictation_payload(event.clone()));
    Err(event.copy.to_string())
}

#[tauri::command]
pub async fn updater_check() -> Result<UpdateStatus, String> {
    let check = tauri::async_runtime::spawn_blocking(|| {
        harbor_updater::check_latest(fetch_url, env!("CARGO_PKG_VERSION"))
    })
    .await
    .unwrap_or_else(|_| harbor_updater::UpdateCheck::unavailable());
    Ok(UpdateStatus {
        available: check.available,
        version: check.version,
    })
}

/// Downloads the newer release's platform artifact and its `.minisig`, then
/// verifies the signature against the baked key. Only a verified artifact is
/// written, to the app-support `updates/` directory; the returned path is what
/// a UI would reveal or hand to the OS installer.
#[tauri::command]
pub async fn updater_install() -> Result<String, String> {
    let check = tauri::async_runtime::spawn_blocking(|| {
        harbor_updater::check_latest(fetch_url, env!("CARGO_PKG_VERSION"))
    })
    .await
    .unwrap_or_else(|_| harbor_updater::UpdateCheck::unavailable());
    if !check.available {
        return Err("no signed update is available".into());
    }
    // The staged file keeps the name the signed manifest declared — not
    // whatever a redirect or URL tail might suggest.
    let name = check
        .file_name
        .clone()
        .filter(|name| {
            // Whitelist a portable filename: separators, drive qualifiers, and
            // dot-only names can never reach the staged path on any OS.
            !name.is_empty()
                && name.chars().any(|ch| ch != '.')
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        })
        .unwrap_or_else(|| "harbor-update".into());
    let dir = crate::application_data_root().join("updates");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(&name);
    // Stream into a temp file while the digest is verified: a crash mid-write
    // or a hash mismatch can never leave a bad artifact at the staged path.
    let staging = dir.join(format!(".{name}.download"));
    let staging_path = staging.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let written = std::fs::File::create(&staging_path)
            .map_err(|error| error.to_string())
            .and_then(|mut file| {
                harbor_updater::install_release(&check, fetch_stream, &mut file)
                    .map_err(|error| error.to_string())
            });
        if written.is_err() {
            let _ = std::fs::remove_file(&staging_path);
        }
        written
    })
    .await
    .map_err(|error| error.to_string())??;
    std::fs::rename(&staging, &path).map_err(|error| error.to_string())?;
    // One staged artifact is enough; older downloads and interrupted temp
    // files are removed once the new one is safely in place.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate != path && candidate.is_file() {
                let _ = std::fs::remove_file(candidate);
            }
        }
    }
    Ok(path.display().to_string())
}

/// Reveal a staged update in the OS file manager. The renderer hands back the
/// path `updater_install` returned; anything outside `updates/` is refused so
/// this never opens an arbitrary location.
#[tauri::command]
pub fn updater_reveal(path: String) -> Result<(), String> {
    let dir = crate::application_data_root()
        .join("updates")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let target = Path::new(&path)
        .canonicalize()
        .map_err(|_| "no staged update at that path".to_string())?;
    if !target.is_file() || !target.starts_with(&dir) {
        return Err("path is not a staged update".into());
    }
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/open");
        command.arg("-R").arg(&target);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new(windows_dir().join("explorer.exe"));
        command.arg(format!("/select,{}", target.display()));
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = {
        let mut command = Command::new(system_helper(
            &[
                "/usr/bin/xdg-open",
                "/usr/local/bin/xdg-open",
                "/bin/xdg-open",
            ],
            "xdg-open",
        ));
        command.arg(target.parent().unwrap_or(dir.as_path()));
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Reading what a CLI already wrote is cheap and touches no network, so this
/// stays a plain command the UI can poll when the builder asks it to.
#[tauri::command]
pub fn engine_usage() -> Vec<harbor_core::usage::EngineUsage> {
    harbor_core::commands::engine_usage(&crate::usage_dir())
}

#[tauri::command]
pub fn usage_bridge_status() -> Result<crate::usage_bridge::BridgeStatus, String> {
    let settings =
        crate::usage_bridge::settings_path().ok_or("could not resolve the home directory")?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    Ok(crate::usage_bridge::status(
        &crate::usage_dir(),
        &settings,
        &exe,
    ))
}

/// Editing a file another application owns is the builder's call, never a side
/// effect of opening a pane, so this only ever runs from the Settings toggle.
#[tauri::command]
pub fn usage_bridge_connect(connected: bool) -> Result<crate::usage_bridge::BridgeStatus, String> {
    let usage_dir = crate::usage_dir();
    let settings =
        crate::usage_bridge::settings_path().ok_or("could not resolve the home directory")?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    if !connected {
        return crate::usage_bridge::disconnect(&usage_dir, &settings, &exe);
    }
    crate::usage_bridge::connect(&usage_dir, &settings, &exe)
}

#[tauri::command]
pub async fn git_diff(
    pool: State<'_, SqlitePool>,
    workspace_id: String,
) -> Result<Vec<FileDiff>, String> {
    harbor_core::commands::git_diff(&pool, workspace_id)
        .await
        .map_err(map_err)
}

/// Only http(s) URLs leave the host through the system URL handler. The
/// renderer never gets `shell:` permissions.
fn allowed_external_url(url: &str) -> Option<&str> {
    let url = url.trim();
    if url.is_empty()
        || url
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'"')
    {
        return None;
    }
    let rest = if url
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        &url[8..]
    } else if url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
    {
        &url[7..]
    } else {
        return None;
    };
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(url)
}

fn open_in_system_browser(url: &str) -> Result<(), String> {
    let status = system_open_command(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("could not open the system browser".into())
    }
}

/// System helpers resolve by absolute path first: PATH can carry a
/// user-writable entry, and a planted `open`/`xdg-open` would run with
/// Harbor's privileges on every external link. The bare-name fallback keeps
/// unknown layouts (NixOS, custom prefixes) working.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_helper(known: &[&str], fallback: &str) -> std::path::PathBuf {
    known
        .iter()
        .map(std::path::PathBuf::from)
        .find(|path| path.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from(fallback))
}

#[cfg(windows)]
fn windows_dir() -> std::path::PathBuf {
    std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
}

#[cfg(target_os = "macos")]
fn system_open_command(url: &str) -> Command {
    let mut command = Command::new("/usr/bin/open");
    command.arg(url);
    command
}

#[cfg(target_os = "linux")]
fn system_open_command(url: &str) -> Command {
    let mut command = Command::new(system_helper(
        &[
            "/usr/bin/xdg-open",
            "/usr/local/bin/xdg-open",
            "/bin/xdg-open",
        ],
        "xdg-open",
    ));
    command.arg(url);
    command
}

#[cfg(windows)]
fn system_open_command(url: &str) -> Command {
    // rundll32 gets the URL as a literal argv entry — no cmd parsing, no %
    // expansion, no `start` title quoting to get wrong.
    let mut command = Command::new(windows_dir().join(r"System32\rundll32.exe"));
    command.arg("url.dll,FileProtocolHandler").arg(url);
    command
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let url = allowed_external_url(&url)
        .ok_or("only http and https URLs can open in the system browser")?;
    open_in_system_browser(url)
}

pub fn handlers() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        settings_get,
        settings_set,
        engines_detect,
        engines_recheck,
        engine_install_adapter,
        workspace_list,
        workspace_add,
        workspace_pick_folder,
        workspace_remove,
        workspace_pin,
        workspace_rename,
        workspace_save_layout,
        workspace_tidy,
        workspace_ensure_tab,
        workspace_configure_tab,
        layout_restore,
        pane_create,
        pane_close,
        pane_set_engine,
        crate::pty_host::pty_spawn,
        crate::pty_host::pty_write_b64,
        crate::pty_host::pty_resize,
        crate::pty_host::pty_pause,
        crate::pty_host::pty_resume,
        crate::pty_host::pty_kill,
        fs_read,
        fs_write,
        fs_list,
        thread_list,
        thread_list_all,
        thread_mark_read,
        thread_create,
        thread_history,
        thread_rename,
        thread_delete,
        thread_pin,
        thread_send,
        thread_cancel,
        thread_set_config,
        thread_set_engine,
        thread_config_options,
        thread_attach_files,
        agent_list,
        agent_create,
        agent_update,
        agent_delete,
        agent_draft_with_ai,
        agent_chat_list,
        agent_chat_create,
        agent_chat_history,
        agent_chat_send,
        agent_chat_cancel,
        agent_chat_set_config,
        agent_chat_config_options,
        agent_chat_rename,
        agent_chat_delete,
        memory_list,
        memory_upsert,
        memory_delete,
        places_list,
        places_grant,
        places_revoke,
        session_search,
        mail_send,
        default_profile_name,
        engine_icons,
        notifications_list,
        session_watch,
        notifications_unread_count,
        notifications_mark_read,
        notifications_clear,
        notification_record,
        face_preview,
        acp_permission_resolve,
        plugin_list,
        plugin_connect,
        plugin_configure,
        plugin_disconnect,
        plugin_set_agent_grant,
        plugin_resolve_approval,
        plugin_approvals_list,
        plugin_grants_list,
        dictation_begin,
        dictation_end,
        dictation_devices,
        dictation_prepare_model,
        updater_check,
        updater_install,
        updater_reveal,
        git_diff,
        open_external_url,
        engine_usage,
        usage_bridge_status,
        usage_bridge_connect
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_pick_is_redeemed_exactly_once() {
        let picks = super::PendingPicks::default();
        let id = picks.insert("/tmp/project");
        assert_eq!(picks.take(&id).unwrap(), "/tmp/project");
        assert!(picks.take(&id).is_err(), "a second redemption must fail");
    }

    #[test]
    fn fabricated_pick_ids_never_resolve() {
        let picks = super::PendingPicks::default();
        picks.insert("/tmp/project");
        assert!(picks.take("not-a-real-pick").is_err());
        assert!(picks.take("").is_err());
    }

    #[test]
    fn allowed_external_url_accepts_http_and_https_only() {
        assert_eq!(
            super::allowed_external_url("https://example.com"),
            Some("https://example.com")
        );
        assert_eq!(super::allowed_external_url("file:///etc/passwd"), None);
        assert_eq!(super::allowed_external_url("javascript:alert(1)"), None);
    }

    #[test]
    fn live_notification_skips_emit_only_when_explicitly_disabled() {
        use serde_json::json;
        assert!(super::should_emit_notification(None));
        assert!(super::should_emit_notification(Some(&json!(true))));
        assert!(super::should_emit_notification(Some(&json!(null))));
        assert!(super::should_emit_notification(Some(&json!("false"))));
        assert!(!super::should_emit_notification(Some(&json!(false))));
    }

    #[test]
    fn kind_filter_defaults_to_all_and_honors_an_explicit_list() {
        use harbor_core::notifications::kind_allowed;
        use serde_json::json;

        assert!(kind_allowed(None, "permission"));
        assert!(kind_allowed(Some(json!(null)), "mail"));
        assert!(kind_allowed(Some(json!(true)), "terminal-exit"));
        assert!(kind_allowed(Some(json!("mail")), "mail"));
        assert!(kind_allowed(Some(json!(["mail"])), "mail"));
        assert!(!kind_allowed(Some(json!(["mail"])), "permission"));
        assert!(!kind_allowed(Some(json!([])), "turn-finished"));
        assert!(kind_allowed(
            Some(json!(["turn-cancelled", "turn-refused", "turn-stopped"])),
            "turn-stopped"
        ));
        assert!(!kind_allowed(
            Some(json!(["turn-cancelled"])),
            "turn-stopped"
        ));
        assert!(!kind_allowed(
            Some(json!([1, true, {"kind": "mail"}])),
            "mail"
        ));
    }

    fn handler_commands() -> Vec<String> {
        let source = include_str!("ipc.rs");
        let body = source
            .split_once("tauri::generate_handler![")
            .expect("handler list")
            .1
            .split_once(']')
            .expect("handler list end")
            .0;
        body.split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.rsplit("::").next().unwrap_or(entry).to_string())
            .collect()
    }

    /// A command is only reachable if it appears in three places: this handler
    /// list, the manifest in build.rs that mints its permission, and the main
    /// capability that grants it. Miss one and the renderer finds out at run
    /// time, with "not allowed" or "command not found".
    #[test]
    fn every_command_is_declared_in_the_manifest_and_the_capability() {
        let manifest = include_str!("../build.rs");
        let capability = include_str!("../capabilities/default.json");

        let mut ungranted = Vec::new();
        let mut unminted = Vec::new();
        for command in handler_commands() {
            if !capability.contains(&format!("\"allow-{}\"", command.replace('_', "-"))) {
                ungranted.push(command.clone());
            }
            if !manifest.contains(&format!("\"{command}\"")) {
                unminted.push(command);
            }
        }

        assert!(
            ungranted.is_empty(),
            "missing from capabilities/default.json: {ungranted:?}"
        );
        assert!(unminted.is_empty(), "missing from build.rs: {unminted:?}");
    }

    fn camel(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        let mut upper = false;
        for ch in name.chars() {
            if ch == '_' {
                upper = true;
            } else if upper {
                out.extend(ch.to_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// The argument names each `#[tauri::command]` expects, as the webview
    /// spells them. Tauri camel-cases them on the way in, and the managed state
    /// it injects itself is not part of the payload.
    fn host_arguments() -> Vec<(String, Vec<String>)> {
        let mut out = Vec::new();
        for source in [include_str!("ipc.rs"), include_str!("pty_host.rs")] {
            for chunk in source.split("#[tauri::command]").skip(1) {
                let Some(header) = chunk.split_once('{').map(|(header, _)| header) else {
                    continue;
                };
                let Some((before, params)) = header.split_once('(') else {
                    continue;
                };
                let Some(name) = before.split_whitespace().last() else {
                    continue;
                };
                let params = params
                    .rsplit_once(')')
                    .map(|(params, _)| params)
                    .unwrap_or("");
                let mut args = Vec::new();
                let mut depth = 0usize;
                let mut current = String::new();
                for ch in params.chars() {
                    match ch {
                        '<' | '(' | '[' => depth += 1,
                        '>' | ')' | ']' => depth = depth.saturating_sub(1),
                        ',' if depth == 0 => {
                            args.push(std::mem::take(&mut current));
                            continue;
                        }
                        _ => {}
                    }
                    current.push(ch);
                }
                args.push(current);
                let args = args
                    .iter()
                    .filter_map(|param| param.split_once(':'))
                    .filter(|(_, ty)| {
                        !ty.contains("State<")
                            && !ty.contains("AppHandle")
                            && !ty.contains("Window")
                    })
                    .map(|(name, _)| camel(name.trim()))
                    .collect::<Vec<_>>();
                out.push((name.to_string(), args));
            }
        }
        out
    }

    /// The `HarborCommands` entries, as `(command, Some(argument names))`, with
    /// `None` for a command the contract declares as taking no arguments.
    fn contract_arguments() -> Vec<(String, Option<Vec<String>>)> {
        let source = include_str!("../../../../packages/schema/src/commands.ts");
        let block = source
            .split_once("export interface HarborCommands {")
            .expect("HarborCommands")
            .1;
        let mut flat = String::new();
        for line in block.lines() {
            let line = line.trim();
            if line.starts_with("/*") || line.starts_with('*') || line.starts_with("//") {
                continue;
            }
            if line == "}" && flat.matches('{').count() == flat.matches('}').count() {
                break;
            }
            flat.push_str(line);
            flat.push(' ');
        }

        let chars: Vec<char> = flat.chars().collect();
        let mut out = Vec::new();
        let mut index = 0usize;
        while index < chars.len() {
            while index < chars.len() && !chars[index].is_ascii_alphabetic() {
                index += 1;
            }
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            if start == index {
                continue;
            }
            let name: String = chars[start..index].iter().collect();
            while index < chars.len() && chars[index] == ' ' {
                index += 1;
            }
            if index >= chars.len() || chars[index] != ':' {
                continue;
            }
            index += 1;
            while index < chars.len() && chars[index] == ' ' {
                index += 1;
            }
            if index >= chars.len() || chars[index] != '{' {
                continue;
            }
            let (body, next) = balanced(&chars, index);
            index = next;
            out.push((name, entry_arguments(&body)));
        }
        out
    }

    /// The text inside the braces at `open`, and the index just past them.
    fn balanced(chars: &[char], open: usize) -> (String, usize) {
        let mut depth = 0usize;
        let mut index = open;
        while index < chars.len() {
            match chars[index] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return (chars[open + 1..index].iter().collect(), index + 1);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        (String::new(), chars.len())
    }

    fn entry_arguments(body: &str) -> Option<Vec<String>> {
        let args = body.split_once("args:")?.1.trim_start().to_string();
        if args.starts_with("undefined") {
            return None;
        }
        let chars: Vec<char> = args.chars().collect();
        let (fields, _) = balanced(&chars, 0);
        let mut names = Vec::new();
        let mut depth = 0usize;
        let mut field = String::new();
        for ch in fields.chars() {
            match ch {
                '{' | '(' | '<' => depth += 1,
                '}' | ')' | '>' => depth = depth.saturating_sub(1),
                ';' if depth == 0 => {
                    names.push(std::mem::take(&mut field));
                    continue;
                }
                _ => {}
            }
            field.push(ch);
        }
        names.push(field);
        Some(
            names
                .iter()
                .filter_map(|field| field.split_once(':'))
                .map(|(name, _)| name.trim().trim_end_matches('?').to_string())
                .collect(),
        )
    }

    /// The webview only reaches the host through `HarborCommands`, so a command
    /// the contract does not list cannot be called and an argument it spells
    /// differently arrives as `null`. Both used to be caught by hand, and both
    /// have been missed. Keep the two files saying the same thing.
    #[test]
    fn the_typescript_contract_matches_the_registered_commands() {
        let registered = handler_commands();
        let contract = contract_arguments();
        let host = host_arguments();

        let listed: Vec<&String> = contract.iter().map(|(name, _)| name).collect();
        let missing: Vec<&String> = registered
            .iter()
            .filter(|command| !listed.contains(command))
            .collect();
        let extra: Vec<&&String> = listed
            .iter()
            .filter(|command| !registered.contains(command))
            .collect();
        assert!(
            missing.is_empty(),
            "registered but missing from packages/schema/src/commands.ts: {missing:?}"
        );
        assert!(
            extra.is_empty(),
            "in packages/schema/src/commands.ts but not registered: {extra:?}"
        );

        let mut mismatched = Vec::new();
        for (command, declared) in &contract {
            let Some((_, expected)) = host.iter().find(|(name, _)| name == command) else {
                continue;
            };
            let mut declared = declared.clone().unwrap_or_default();
            let mut expected = expected.clone();
            declared.sort();
            expected.sort();
            if declared != expected {
                mismatched.push(format!(
                    "{command}: contract {declared:?} host {expected:?}"
                ));
            }
        }
        assert!(
            mismatched.is_empty(),
            "argument names differ — {mismatched:?}"
        );
    }
}
