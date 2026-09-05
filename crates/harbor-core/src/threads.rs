use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::Error,
    types::{ContentPart, ThreadRecord},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn list(
    pool: &SqlitePool,
    workspace_id: Option<&str>,
) -> Result<Vec<ThreadRecord>, Error> {
    let rows = match workspace_id {
        Some(workspace_id) => {
            sqlx::query_as::<_, (String, Option<String>, String, String, i64, i64)>(
                "SELECT id, workspace_id, title, engine_id, pinned, unread FROM threads
                 WHERE workspace_id = ?1 ORDER BY pinned DESC, updated_at DESC",
            )
            .bind(workspace_id)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, (String, Option<String>, String, String, i64, i64)>(
                "SELECT id, workspace_id, title, engine_id, pinned, unread FROM threads
                 WHERE workspace_id IS NULL ORDER BY pinned DESC, updated_at DESC",
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows
        .into_iter()
        .map(
            |(id, workspace_id, title, engine_id, pinned, unread)| ThreadRecord {
                id,
                workspace_id,
                title,
                engine_id,
                pinned: pinned != 0,
                unread: unread != 0,
            },
        )
        .collect())
}

pub async fn create(
    pool: &SqlitePool,
    workspace_id: Option<String>,
    engine_id: String,
) -> Result<ThreadRecord, Error> {
    if engine_id.trim().is_empty() {
        return Err(Error::Message("engine_id required".into()));
    }
    let id = Uuid::now_v7().to_string();
    let ts = now();
    sqlx::query(
        "INSERT INTO threads (id, workspace_id, title, engine_id, pinned, unread, config_json, extra_roots_json, created_at, updated_at)
         VALUES (?1, ?2, 'New thread', ?3, 0, 0, '{}', '[]', ?4, ?4)",
    )
    .bind(&id)
    .bind(&workspace_id)
    .bind(&engine_id)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(ThreadRecord {
        id,
        workspace_id,
        title: "New thread".into(),
        engine_id,
        pinned: false,
        unread: false,
    })
}

pub async fn rename(pool: &SqlitePool, id: &str, title: &str) -> Result<(), Error> {
    let result = sqlx::query("UPDATE threads SET title = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(title)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("thread not found".into()));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM threads WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn pin(pool: &SqlitePool, id: &str, pinned: bool) -> Result<(), Error> {
    sqlx::query("UPDATE threads SET pinned = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(i64::from(pinned))
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn grant_root(pool: &SqlitePool, id: &str, path: &str) -> Result<(), Error> {
    let (extra,): (String,) = sqlx::query_as("SELECT extra_roots_json FROM threads WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("thread not found".into()))?;
    let mut roots: Vec<String> = serde_json::from_str(&extra).unwrap_or_default();
    if !roots.iter().any(|root| root == path) {
        roots.push(path.into());
    }
    sqlx::query("UPDATE threads SET extra_roots_json = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(serde_json::to_string(&roots)?)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    let _ = json!(roots);
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ThreadContext {
    pub id: String,
    pub engine_id: String,
    pub workspace_id: Option<String>,
    pub workspace_folder: Option<String>,
    pub extra_roots: Vec<String>,
    pub acp_session: Option<String>,
}

type ThreadRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

pub async fn context(pool: &SqlitePool, id: &str) -> Result<ThreadContext, Error> {
    let row: Option<ThreadRow> = sqlx::query_as(
        "SELECT t.id, t.engine_id, t.workspace_id, w.folder, t.extra_roots_json, t.acp_session
         FROM threads t
         LEFT JOIN workspaces w ON w.id = t.workspace_id
         WHERE t.id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let (id, engine_id, workspace_id, workspace_folder, extra, acp_session) =
        row.ok_or_else(|| Error::Message("thread not found".into()))?;
    Ok(ThreadContext {
        id,
        engine_id,
        workspace_id,
        workspace_folder,
        extra_roots: serde_json::from_str(&extra).unwrap_or_default(),
        acp_session,
    })
}

pub async fn set_acp_session(pool: &SqlitePool, id: &str, session: &str) -> Result<(), Error> {
    sqlx::query("UPDATE threads SET acp_session = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(session)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn append_message(
    pool: &SqlitePool,
    chat_id: &str,
    chat_kind: &str,
    role: &str,
    prose: &str,
) -> Result<(), Error> {
    let message_id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO messages (id, chat_id, chat_kind, role, prose, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)",
    )
    .bind(&message_id)
    .bind(chat_id)
    .bind(chat_kind)
    .bind(role)
    .bind(prose)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn send(pool: &SqlitePool, id: &str, parts: &[ContentPart]) -> Result<(), Error> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM threads WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(Error::Message("thread not found".into()));
    }
    let prose = parts
        .iter()
        .filter_map(|part| part.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    append_message(pool, id, "thread", "user", &prose).await?;
    sqlx::query("UPDATE threads SET updated_at = ?1, unread = 0 WHERE id = ?2")
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::ContentPart;

    #[tokio::test]
    async fn send_persists_user_and_context() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let thread = create(&pool, None, "opencode".into()).await.unwrap();
        send(
            &pool,
            &thread.id,
            &[ContentPart {
                r#type: "text".into(),
                text: Some("hi".into()),
                path: None,
            }],
        )
        .await
        .unwrap();
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
        append_message(&pool, &thread.id, "thread", "assistant", "ok")
            .await
            .unwrap();
        set_acp_session(&pool, &thread.id, "sess-1").await.unwrap();
        let ctx = context(&pool, &thread.id).await.unwrap();
        assert_eq!(ctx.engine_id, "opencode");
        assert_eq!(ctx.acp_session.as_deref(), Some("sess-1"));
    }
}
