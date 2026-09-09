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
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;

fn map_err(error: harbor_core::error::Error) -> String {
    error.to_string()
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

fn allow_workspace_directory(app: &AppHandle, folder: &str) {
    // The native picker and the persisted workspace list both feed the
    // Tauri filesystem scope. The custom Rust commands still enforce their
    // own root check, but this keeps plugin-backed file APIs usable too.
    let _ = app.fs_scope().allow_directory(Path::new(folder), true);
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

/// The native picker selects a folder and grants its recursive filesystem scope.
#[tauri::command]
pub async fn workspace_pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a workspace folder")
            .blocking_pick_folder()
            .map(|path| {
                path.into_path()
                    .map(|path| {
                        let value = path.to_string_lossy().into_owned();
                        allow_workspace_directory(&app, &value);
                        value
                    })
                    .map_err(|error| error.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn workspace_add(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    folder: String,
) -> Result<Workspace, String> {
    let workspace = harbor_core::commands::workspace_add(&pool, folder)
        .await
        .map_err(map_err)?;
    allow_workspace_directory(&app, &workspace.folder);
    Ok(workspace)
}

#[tauri::command]
pub async fn workspace_remove(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::workspace_remove(&pool, id)
        .await
        .map_err(map_err)
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
pub async fn thread_grant_root(
    pool: State<'_, SqlitePool>,
    id: String,
    path: String,
) -> Result<(), String> {
    harbor_core::commands::thread_grant_root(&pool, id, path)
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
    pool: State<'_, SqlitePool>,
    input: CreateAgent,
) -> Result<AgentRecord, String> {
    harbor_core::commands::agent_create(&pool, input)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_update(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    input: UpdateAgent,
) -> Result<(), String> {
    let home = input.home_path.clone();
    harbor_core::commands::agent_update(&pool, input)
        .await
        .map_err(map_err)?;
    if let Some(path) = home.filter(|path| !path.trim().is_empty()) {
        allow_workspace_directory(&app, &path);
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_delete(
    pool: State<'_, SqlitePool>,
    registry: State<'_, AcpRegistry>,
    id: String,
) -> Result<(), String> {
    crate::acp_host::drop_agent_sessions(&pool, &registry, &id).await;
    harbor_core::commands::agent_delete(&pool, id)
        .await
        .map_err(map_err)
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
    agent_id: String,
    path: String,
) -> Result<(), String> {
    allow_workspace_directory(&app, &path);
    harbor_core::commands::places_grant(&pool, agent_id, path)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn places_revoke(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::places_revoke(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn session_search(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    query: String,
) -> Result<Vec<SearchHit>, String> {
    harbor_core::commands::session_search(&pool, agent_id, query)
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

/// Records an event and tells the renderer, so the bell updates without polling.
pub async fn notify(
    app: &AppHandle,
    pool: &SqlitePool,
    kind: &str,
    title: &str,
    body: &str,
    target: harbor_core::notifications::Target,
) {
    if let Ok(row) = harbor_core::notifications::record(pool, kind, title, body, target).await {
        let _ = app.emit("notification", row);
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

#[tauri::command]
pub async fn updater_install() -> Result<(), String> {
    Err(harbor_updater::refuse_install().to_string())
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
        thread_grant_root,
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
        git_diff
    ]
}

#[cfg(test)]
mod tests {
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
