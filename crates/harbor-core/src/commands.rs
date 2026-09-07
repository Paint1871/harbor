//! Host commands. None of these inspect tokens, entitlements, or the network
//! before returning; absence of a cloud session is success.

use serde_json::Value;
use sqlx::SqlitePool;

use crate::{
    error::Error,
    settings,
    types::{
        AgentChat, AgentRecord, ChatMessage, CreateAgent, DetectedEngine, FileDiff, FsEntry,
        Memory, Notification, PaneLayout, PaneState, Place, PluginApproval, PluginGrant, PluginRow,
        SearchHit, ThreadRecord, UpdateAgent, UpdateStatus, Workspace, WorkspaceSetup,
        WorkspaceTab,
    },
};

/// Suggested only. Nothing writes it until the builder confirms Start local.
pub fn default_profile_name() -> String {
    settings::os_account_name()
}

pub async fn settings_get(pool: &SqlitePool, key: &str) -> Result<Value, Error> {
    Ok(settings::get(pool, key).await?.unwrap_or(Value::Null))
}

pub async fn settings_set(pool: &SqlitePool, key: &str, value: Value) -> Result<(), Error> {
    settings::set(pool, key, &value).await
}

pub async fn engines_detect(_pool: &SqlitePool) -> Result<Vec<DetectedEngine>, Error> {
    Ok(crate::engines::recheck())
}

pub async fn engines_recheck(_pool: &SqlitePool) -> Result<Vec<DetectedEngine>, Error> {
    Ok(crate::engines::recheck())
}

pub async fn workspace_list(pool: &SqlitePool) -> Result<Vec<Workspace>, Error> {
    crate::workspaces::list(pool).await
}

pub async fn workspace_add(pool: &SqlitePool, folder: String) -> Result<Workspace, Error> {
    crate::workspaces::add(pool, folder).await
}

pub async fn workspace_ensure_tab(
    pool: &SqlitePool,
    workspace_id: String,
) -> Result<WorkspaceTab, Error> {
    crate::layout::ensure_default_tab(pool, &workspace_id).await
}

pub async fn workspace_configure_tab(
    pool: &SqlitePool,
    workspace_id: String,
    setup: WorkspaceSetup,
) -> Result<WorkspaceTab, Error> {
    crate::layout::configure_workspace_tab(pool, &workspace_id, &setup).await
}

pub async fn workspace_remove(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::workspaces::remove(pool, &id).await
}

pub async fn workspace_pin(pool: &SqlitePool, id: String, pinned: bool) -> Result<(), Error> {
    crate::workspaces::pin(pool, &id, pinned).await
}

pub async fn workspace_rename(
    pool: &SqlitePool,
    id: String,
    title: String,
) -> Result<Workspace, Error> {
    crate::workspaces::rename(pool, &id, &title).await
}

pub async fn workspace_save_layout(
    pool: &SqlitePool,
    tab_id: String,
    layout: PaneLayout,
) -> Result<(), Error> {
    crate::layout::save(pool, &tab_id, &layout).await
}

pub async fn workspace_tidy(pool: &SqlitePool, tab_id: String) -> Result<PaneLayout, Error> {
    crate::layout::tidy(pool, &tab_id).await
}

pub async fn layout_restore(pool: &SqlitePool) -> Result<Vec<WorkspaceTab>, Error> {
    crate::layout::restore(pool).await
}

pub async fn pane_create(
    pool: &SqlitePool,
    tab_id: String,
    kind: String,
    state: PaneState,
) -> Result<String, Error> {
    crate::layout::pane_create(pool, &tab_id, &kind, &state).await
}

pub async fn pane_close(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::layout::pane_close(pool, &id).await
}

pub async fn pane_set_engine(
    pool: &SqlitePool,
    id: String,
    engine_id: String,
) -> Result<(), Error> {
    crate::layout::set_engine(pool, &id, &engine_id).await
}

pub async fn pty_spawn(
    _pool: &SqlitePool,
    _pane_id: String,
    _workspace_id: String,
    _cols: u16,
    _rows: u16,
    _shell: Option<String>,
    _engine_id: Option<String>,
) -> Result<(), Error> {
    Err(Error::unimplemented("pty_spawn"))
}

pub async fn pty_write_b64(_pane_id: String, _b64: String) -> Result<(), Error> {
    Err(Error::unimplemented("pty_write_b64"))
}

pub async fn pty_resize(_pane_id: String, _cols: u16, _rows: u16) -> Result<(), Error> {
    Err(Error::unimplemented("pty_resize"))
}

pub async fn pty_pause(_pane_id: String) -> Result<(), Error> {
    Err(Error::unimplemented("pty_pause"))
}

pub async fn pty_resume(_pane_id: String) -> Result<(), Error> {
    Err(Error::unimplemented("pty_resume"))
}

pub async fn pty_kill(_pane_id: String) -> Result<(), Error> {
    Err(Error::unimplemented("pty_kill"))
}

pub async fn fs_read(
    pool: &SqlitePool,
    workspace_id: String,
    path: String,
) -> Result<String, Error> {
    crate::files::read(pool, &workspace_id, &path).await
}

pub async fn fs_write(
    pool: &SqlitePool,
    workspace_id: String,
    path: String,
    contents: String,
) -> Result<(), Error> {
    crate::files::write(pool, &workspace_id, &path, &contents).await
}

pub async fn fs_list(
    pool: &SqlitePool,
    workspace_id: String,
    path: String,
) -> Result<Vec<FsEntry>, Error> {
    crate::files::list(pool, &workspace_id, &path).await
}

pub async fn thread_list(
    pool: &SqlitePool,
    workspace_id: Option<String>,
) -> Result<Vec<ThreadRecord>, Error> {
    crate::threads::list(pool, workspace_id.as_deref()).await
}

pub async fn thread_create(
    pool: &SqlitePool,
    workspace_id: Option<String>,
    engine_id: String,
) -> Result<ThreadRecord, Error> {
    crate::threads::create(pool, workspace_id, engine_id).await
}

pub async fn thread_rename(pool: &SqlitePool, id: String, title: String) -> Result<(), Error> {
    crate::threads::rename(pool, &id, &title).await
}

pub async fn thread_delete(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::threads::delete(pool, &id).await
}

pub async fn thread_pin(pool: &SqlitePool, id: String, pinned: bool) -> Result<(), Error> {
    crate::threads::pin(pool, &id, pinned).await
}

pub async fn thread_send(
    pool: &SqlitePool,
    id: String,
    parts: Vec<crate::types::ContentPart>,
) -> Result<(), Error> {
    crate::threads::send(pool, &id, &parts).await
}

/// Host cancels the ACP turn; core is a successful no-op so IPC fallback works.
pub async fn thread_cancel(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Ok(())
}

pub async fn thread_set_config(
    pool: &SqlitePool,
    id: String,
    option_id: String,
    value: Value,
) -> Result<(), Error> {
    crate::threads::set_config(pool, &id, &option_id, value).await
}

pub async fn thread_grant_root(pool: &SqlitePool, id: String, path: String) -> Result<(), Error> {
    crate::threads::grant_root(pool, &id, &path).await
}

pub async fn thread_attach_files(
    pool: &SqlitePool,
    id: String,
    paths: Vec<String>,
) -> Result<(), Error> {
    crate::threads::attach_files(pool, &id, &paths).await
}

pub async fn agent_list(pool: &SqlitePool) -> Result<Vec<AgentRecord>, Error> {
    crate::agents::list(pool).await
}

pub async fn agent_create(pool: &SqlitePool, input: CreateAgent) -> Result<AgentRecord, Error> {
    crate::agents::create(pool, input).await
}

pub async fn agent_update(pool: &SqlitePool, input: UpdateAgent) -> Result<(), Error> {
    crate::agents::update(pool, input).await
}

pub async fn agent_delete(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::agents::delete(pool, &id).await
}

pub async fn agent_draft_with_ai(pool: &SqlitePool, hint: String) -> Result<CreateAgent, Error> {
    crate::agents::draft_with_ai(pool, hint).await
}

pub async fn agent_chat_list(pool: &SqlitePool, agent_id: String) -> Result<Vec<AgentChat>, Error> {
    crate::chats::list(pool, &agent_id).await
}

pub async fn agent_chat_create(pool: &SqlitePool, agent_id: String) -> Result<AgentChat, Error> {
    crate::chats::create(pool, &agent_id).await
}

pub async fn agent_chat_history(
    pool: &SqlitePool,
    chat_id: String,
) -> Result<Vec<ChatMessage>, Error> {
    crate::chats::history(pool, &chat_id).await
}

pub async fn agent_chat_send(
    pool: &SqlitePool,
    chat_id: String,
    parts: Vec<crate::types::ContentPart>,
) -> Result<(), Error> {
    crate::chats::send(pool, &chat_id, &parts).await
}

pub async fn agent_chat_cancel(pool: &SqlitePool, chat_id: String) -> Result<(), Error> {
    crate::chats::cancel(pool, &chat_id).await
}

pub async fn agent_chat_set_config(
    pool: &SqlitePool,
    chat_id: String,
    option_id: String,
    value: Value,
) -> Result<(), Error> {
    crate::chats::set_config(pool, &chat_id, &option_id, value).await
}

pub async fn memory_list(pool: &SqlitePool, agent_id: String) -> Result<Vec<Memory>, Error> {
    crate::memory::list(pool, &agent_id).await
}

pub async fn memory_upsert(
    pool: &SqlitePool,
    agent_id: String,
    body: String,
) -> Result<Memory, Error> {
    crate::memory::upsert(pool, &agent_id, &body).await
}

pub async fn memory_delete(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::memory::delete(pool, &id).await
}

pub async fn places_list(pool: &SqlitePool, agent_id: String) -> Result<Vec<Place>, Error> {
    crate::places::list(pool, &agent_id).await
}

pub async fn places_grant(pool: &SqlitePool, agent_id: String, path: String) -> Result<(), Error> {
    crate::places::grant(pool, &agent_id, &path).await
}

pub async fn places_revoke(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::places::revoke(pool, &id).await
}

pub async fn session_search(
    pool: &SqlitePool,
    agent_id: String,
    query: String,
) -> Result<Vec<SearchHit>, Error> {
    crate::search::session_search(pool, &agent_id, &query).await
}

pub async fn mail_send(
    pool: &SqlitePool,
    from_agent_id: String,
    to_agent_id: String,
    body: String,
) -> Result<(), Error> {
    crate::mail::send(pool, &from_agent_id, &to_agent_id, &body).await
}

pub async fn notifications_list(pool: &SqlitePool) -> Result<Vec<Notification>, Error> {
    crate::notifications::list(pool).await
}

pub async fn notifications_unread_count(pool: &SqlitePool) -> Result<i64, Error> {
    crate::notifications::unread_count(pool).await
}

pub async fn notifications_mark_read(pool: &SqlitePool) -> Result<(), Error> {
    crate::notifications::mark_read(pool).await
}

pub async fn face_preview(
    pool: &SqlitePool,
    agent_id: String,
    face_index: i32,
) -> Result<String, Error> {
    crate::agents::face_preview(pool, &agent_id, face_index).await
}

pub async fn acp_permission_resolve(
    pool: &SqlitePool,
    id: String,
    option_id: Option<String>,
    cancelled: bool,
) -> Result<(), Error> {
    crate::acp::permission_resolve(pool, &id, option_id.as_deref(), cancelled).await
}

pub async fn plugin_list(pool: &SqlitePool) -> Result<Vec<PluginRow>, Error> {
    crate::plugins::list(pool).await
}

pub async fn plugin_connect(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::plugins::connect(pool, &id).await
}

pub async fn plugin_mark_connected(
    pool: &SqlitePool,
    id: &str,
    display_name: &str,
    account_label: Option<&str>,
) -> Result<(), Error> {
    crate::plugins::mark_connected(pool, id, display_name, account_label).await
}

pub async fn plugin_mark_disconnected(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    crate::plugins::mark_disconnected(pool, id).await
}

pub async fn plugin_disconnect(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::plugins::disconnect(pool, &id).await
}

pub async fn plugin_set_agent_grant(
    pool: &SqlitePool,
    agent_id: String,
    plugin_id: String,
    enabled: bool,
) -> Result<(), Error> {
    crate::plugins::set_agent_grant(pool, &agent_id, &plugin_id, enabled).await
}

pub async fn plugin_resolve_approval(
    pool: &SqlitePool,
    id: String,
    allow: bool,
) -> Result<(), Error> {
    crate::plugins::resolve_approval(pool, &id, allow).await
}

pub async fn plugin_approvals_list(pool: &SqlitePool) -> Result<Vec<PluginApproval>, Error> {
    crate::plugins::list_approvals(pool).await
}

pub async fn plugin_grants_list(
    pool: &SqlitePool,
    agent_id: String,
) -> Result<Vec<PluginGrant>, Error> {
    crate::plugins::list_grants(pool, &agent_id).await
}

pub async fn dictation_begin() -> Result<(), Error> {
    Err(Error::unimplemented("dictation_begin"))
}

pub async fn dictation_end() -> Result<(), Error> {
    Err(Error::unimplemented("dictation_end"))
}

pub async fn dictation_devices() -> Result<Vec<Value>, Error> {
    Err(Error::unimplemented("dictation_devices"))
}

pub async fn dictation_prepare_model() -> Result<(), Error> {
    Err(Error::unimplemented("dictation_prepare_model"))
}

pub async fn updater_check() -> Result<UpdateStatus, Error> {
    Err(Error::unimplemented("updater_check"))
}

pub async fn updater_install() -> Result<(), Error> {
    Err(Error::unimplemented("updater_install"))
}

pub async fn git_diff(pool: &SqlitePool, workspace_id: String) -> Result<Vec<FileDiff>, Error> {
    let (folder,): (String,) = sqlx::query_as("SELECT folder FROM workspaces WHERE id = ?1")
        .bind(&workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("workspace not found".into()))?;
    Ok(harbor_git::unified_diffs(&folder)?
        .into_iter()
        .map(|diff| FileDiff {
            path: diff.path,
            patch: diff.patch,
        })
        .collect())
}
