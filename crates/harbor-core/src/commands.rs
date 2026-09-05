//! Host commands. None of these inspect tokens, entitlements, or the network
//! before returning; absence of a cloud session is success.

use serde_json::Value;
use sqlx::SqlitePool;

use crate::{
    error::Error,
    settings,
    types::{
        AgentChat, AgentRecord, CreateAgent, DetectedEngine, FileDiff, FsEntry, Memory, PaneLayout,
        PaneState, PluginRow, SearchHit, ThreadRecord, UpdateAgent, UpdateStatus, Workspace,
    },
};

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

pub async fn workspace_remove(pool: &SqlitePool, id: String) -> Result<(), Error> {
    crate::workspaces::remove(pool, &id).await
}

pub async fn workspace_pin(pool: &SqlitePool, id: String, pinned: bool) -> Result<(), Error> {
    crate::workspaces::pin(pool, &id, pinned).await
}

pub async fn workspace_save_layout(
    _pool: &SqlitePool,
    _tab_id: String,
    _layout: PaneLayout,
) -> Result<(), Error> {
    Err(Error::unimplemented("workspace_save_layout"))
}

pub async fn workspace_tidy(_pool: &SqlitePool, _tab_id: String) -> Result<PaneLayout, Error> {
    Err(Error::unimplemented("workspace_tidy"))
}

pub async fn layout_restore(_pool: &SqlitePool) -> Result<(), Error> {
    Err(Error::unimplemented("layout_restore"))
}

pub async fn pane_create(
    _pool: &SqlitePool,
    _tab_id: String,
    _kind: String,
    _state: PaneState,
) -> Result<String, Error> {
    Err(Error::unimplemented("pane_create"))
}

pub async fn pane_close(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("pane_close"))
}

pub async fn pty_spawn(
    _pool: &SqlitePool,
    _pane_id: String,
    _cwd: String,
    _shell: Option<String>,
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
    _pool: &SqlitePool,
    _workspace_id: String,
    _path: String,
) -> Result<String, Error> {
    Err(Error::unimplemented("fs_read"))
}

pub async fn fs_write(
    _pool: &SqlitePool,
    _workspace_id: String,
    _path: String,
    _contents: String,
) -> Result<(), Error> {
    Err(Error::unimplemented("fs_write"))
}

pub async fn fs_list(
    _pool: &SqlitePool,
    _workspace_id: String,
    _path: String,
) -> Result<Vec<FsEntry>, Error> {
    Err(Error::unimplemented("fs_list"))
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
    _pool: &SqlitePool,
    _id: String,
    _parts: Vec<crate::types::ContentPart>,
) -> Result<(), Error> {
    Err(Error::unimplemented("thread_send"))
}

pub async fn thread_cancel(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("thread_cancel"))
}

pub async fn thread_set_config(
    _pool: &SqlitePool,
    _id: String,
    _option_id: String,
    _value: Value,
) -> Result<(), Error> {
    Err(Error::unimplemented("thread_set_config"))
}

pub async fn thread_grant_root(pool: &SqlitePool, id: String, path: String) -> Result<(), Error> {
    crate::threads::grant_root(pool, &id, &path).await
}

pub async fn thread_attach_files(
    _pool: &SqlitePool,
    _id: String,
    _paths: Vec<String>,
) -> Result<(), Error> {
    Err(Error::unimplemented("thread_attach_files"))
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

pub async fn agent_draft_with_ai(_pool: &SqlitePool, _hint: String) -> Result<CreateAgent, Error> {
    Err(Error::unimplemented("agent_draft_with_ai"))
}

pub async fn agent_chat_list(
    _pool: &SqlitePool,
    _agent_id: String,
) -> Result<Vec<AgentChat>, Error> {
    Err(Error::unimplemented("agent_chat_list"))
}

pub async fn agent_chat_create(_pool: &SqlitePool, _agent_id: String) -> Result<AgentChat, Error> {
    Err(Error::unimplemented("agent_chat_create"))
}

pub async fn agent_chat_send(
    _pool: &SqlitePool,
    _chat_id: String,
    _parts: Vec<crate::types::ContentPart>,
) -> Result<(), Error> {
    Err(Error::unimplemented("agent_chat_send"))
}

pub async fn agent_chat_cancel(_pool: &SqlitePool, _chat_id: String) -> Result<(), Error> {
    Err(Error::unimplemented("agent_chat_cancel"))
}

pub async fn agent_chat_set_config(
    _pool: &SqlitePool,
    _chat_id: String,
    _option_id: String,
    _value: Value,
) -> Result<(), Error> {
    Err(Error::unimplemented("agent_chat_set_config"))
}

pub async fn memory_list(_pool: &SqlitePool, _agent_id: String) -> Result<Vec<Memory>, Error> {
    Err(Error::unimplemented("memory_list"))
}

pub async fn memory_upsert(
    _pool: &SqlitePool,
    _agent_id: String,
    _body: String,
) -> Result<Memory, Error> {
    Err(Error::unimplemented("memory_upsert"))
}

pub async fn memory_delete(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("memory_delete"))
}

pub async fn places_grant(
    _pool: &SqlitePool,
    _agent_id: String,
    _path: String,
) -> Result<(), Error> {
    Err(Error::unimplemented("places_grant"))
}

pub async fn places_revoke(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("places_revoke"))
}

pub async fn session_search(
    _pool: &SqlitePool,
    _agent_id: String,
    _query: String,
) -> Result<Vec<SearchHit>, Error> {
    Err(Error::unimplemented("session_search"))
}

pub async fn mail_send(
    _pool: &SqlitePool,
    _from_agent_id: String,
    _to_agent_id: String,
    _body: String,
) -> Result<(), Error> {
    Err(Error::unimplemented("mail_send"))
}

pub async fn face_preview(
    _pool: &SqlitePool,
    _agent_id: String,
    _face_index: i32,
) -> Result<String, Error> {
    Err(Error::unimplemented("face_preview"))
}

pub async fn acp_permission_resolve(
    _pool: &SqlitePool,
    _id: String,
    _option_id: Option<String>,
    _cancelled: bool,
) -> Result<(), Error> {
    Err(Error::unimplemented("acp_permission_resolve"))
}

pub async fn plugin_list(_pool: &SqlitePool) -> Result<Vec<PluginRow>, Error> {
    Ok(vec![PluginRow {
        id: "github".into(),
        display_name: "GitHub".into(),
        status: "available".into(),
    }])
}

pub async fn plugin_connect(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("plugin_connect"))
}

pub async fn plugin_disconnect(_pool: &SqlitePool, _id: String) -> Result<(), Error> {
    Err(Error::unimplemented("plugin_disconnect"))
}

pub async fn plugin_set_agent_grant(
    _pool: &SqlitePool,
    _agent_id: String,
    _plugin_id: String,
    _enabled: bool,
) -> Result<(), Error> {
    Err(Error::unimplemented("plugin_set_agent_grant"))
}

pub async fn plugin_resolve_approval(
    _pool: &SqlitePool,
    _id: String,
    _allow: bool,
) -> Result<(), Error> {
    Err(Error::unimplemented("plugin_resolve_approval"))
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
