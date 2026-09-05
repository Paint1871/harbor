use harbor_core::SqlitePool;
use harbor_core::types::{
    AgentChat, AgentRecord, ContentPart, CreateAgent, DetectedEngine, FileDiff, FsEntry, Memory,
    PaneLayout, PaneState, PluginRow, SearchHit, ThreadRecord, UpdateAgent, UpdateStatus,
    Workspace,
};
use serde_json::Value;
use tauri::State;

fn map_err(error: harbor_core::error::Error) -> String {
    error.to_string()
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
pub async fn engines_detect(pool: State<'_, SqlitePool>) -> Result<Vec<DetectedEngine>, String> {
    harbor_core::commands::engines_detect(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn engines_recheck(pool: State<'_, SqlitePool>) -> Result<Vec<DetectedEngine>, String> {
    harbor_core::commands::engines_recheck(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_list(pool: State<'_, SqlitePool>) -> Result<Vec<Workspace>, String> {
    harbor_core::commands::workspace_list(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn workspace_add(
    pool: State<'_, SqlitePool>,
    folder: String,
) -> Result<Workspace, String> {
    harbor_core::commands::workspace_add(&pool, folder)
        .await
        .map_err(map_err)
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
pub async fn layout_restore(pool: State<'_, SqlitePool>) -> Result<(), String> {
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
pub async fn pty_spawn(
    pool: State<'_, SqlitePool>,
    pane_id: String,
    cwd: String,
    shell: Option<String>,
) -> Result<(), String> {
    harbor_core::commands::pty_spawn(&pool, pane_id, cwd, shell)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pty_write_b64(pane_id: String, b64: String) -> Result<(), String> {
    harbor_core::commands::pty_write_b64(pane_id, b64)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pty_resize(pane_id: String, cols: u16, rows: u16) -> Result<(), String> {
    harbor_core::commands::pty_resize(pane_id, cols, rows)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pty_pause(pane_id: String) -> Result<(), String> {
    harbor_core::commands::pty_pause(pane_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pty_resume(pane_id: String) -> Result<(), String> {
    harbor_core::commands::pty_resume(pane_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn pty_kill(pane_id: String) -> Result<(), String> {
    harbor_core::commands::pty_kill(pane_id)
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
pub async fn thread_delete(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
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
    pool: State<'_, SqlitePool>,
    id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::thread_send(&pool, id, parts)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_cancel(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::thread_cancel(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn thread_set_config(
    pool: State<'_, SqlitePool>,
    id: String,
    option_id: String,
    value: Value,
) -> Result<(), String> {
    harbor_core::commands::thread_set_config(&pool, id, option_id, value)
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
pub async fn agent_delete(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
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
pub async fn agent_chat_send(
    pool: State<'_, SqlitePool>,
    chat_id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_send(&pool, chat_id, parts)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_cancel(pool: State<'_, SqlitePool>, chat_id: String) -> Result<(), String> {
    harbor_core::commands::agent_chat_cancel(&pool, chat_id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn agent_chat_set_config(
    pool: State<'_, SqlitePool>,
    chat_id: String,
    option_id: String,
    value: Value,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_set_config(&pool, chat_id, option_id, value)
        .await
        .map_err(map_err)
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
    id: String,
    option_id: Option<String>,
    cancelled: bool,
) -> Result<(), String> {
    harbor_core::commands::acp_permission_resolve(&pool, id, option_id, cancelled)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_list(pool: State<'_, SqlitePool>) -> Result<Vec<PluginRow>, String> {
    harbor_core::commands::plugin_list(&pool)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_connect(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::plugin_connect(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_disconnect(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    harbor_core::commands::plugin_disconnect(&pool, id)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_set_agent_grant(
    pool: State<'_, SqlitePool>,
    agent_id: String,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    harbor_core::commands::plugin_set_agent_grant(&pool, agent_id, plugin_id, enabled)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn plugin_resolve_approval(
    pool: State<'_, SqlitePool>,
    id: String,
    allow: bool,
) -> Result<(), String> {
    harbor_core::commands::plugin_resolve_approval(&pool, id, allow)
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn dictation_begin() -> Result<(), String> {
    harbor_core::commands::dictation_begin()
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn dictation_end() -> Result<(), String> {
    harbor_core::commands::dictation_end()
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn dictation_devices() -> Result<Vec<Value>, String> {
    harbor_core::commands::dictation_devices()
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn dictation_prepare_model() -> Result<(), String> {
    harbor_core::commands::dictation_prepare_model()
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn updater_check() -> Result<UpdateStatus, String> {
    harbor_core::commands::updater_check()
        .await
        .map_err(map_err)
}

#[tauri::command]
pub async fn updater_install() -> Result<(), String> {
    harbor_core::commands::updater_install()
        .await
        .map_err(map_err)
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
        workspace_remove,
        workspace_pin,
        workspace_save_layout,
        workspace_tidy,
        layout_restore,
        pane_create,
        pane_close,
        pty_spawn,
        pty_write_b64,
        pty_resize,
        pty_pause,
        pty_resume,
        pty_kill,
        fs_read,
        fs_write,
        fs_list,
        thread_list,
        thread_create,
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
        agent_chat_send,
        agent_chat_cancel,
        agent_chat_set_config,
        memory_list,
        memory_upsert,
        memory_delete,
        places_grant,
        places_revoke,
        session_search,
        mail_send,
        face_preview,
        acp_permission_resolve,
        plugin_list,
        plugin_connect,
        plugin_disconnect,
        plugin_set_agent_grant,
        plugin_resolve_approval,
        dictation_begin,
        dictation_end,
        dictation_devices,
        dictation_prepare_model,
        updater_check,
        updater_install,
        git_diff
    ]
}
