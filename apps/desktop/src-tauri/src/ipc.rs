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
use tauri::{AppHandle, Emitter, State};
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
pub async fn agent_list(pool: State<'_, SqlitePool>) -> Result<Vec<AgentRecord>, String> {
    harbor_core::commands::agent_list(&pool)
        .await
        .map_err(map_err)
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
pub async fn agent_update(pool: State<'_, SqlitePool>, input: UpdateAgent) -> Result<(), String> {
    harbor_core::commands::agent_update(&pool, input)
        .await
        .map_err(map_err)
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
    pool: State<'_, SqlitePool>,
    agent_id: String,
    path: String,
) -> Result<(), String> {
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
