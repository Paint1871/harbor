use std::path::{Path, PathBuf};

use serde_json::{Value, json};
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

/// Read one thread's persisted transcript without starting its engine.
pub async fn history(pool: &SqlitePool, id: &str) -> Result<Vec<crate::types::ChatMessage>, Error> {
    context(pool, id).await?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, role, prose FROM messages WHERE chat_kind = 'thread' AND chat_id = ?1 ORDER BY created_at, rowid",
    ).bind(id).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, role, text)| crate::types::ChatMessage { id, role, text })
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
    Ok(())
}

pub async fn set_config(
    pool: &SqlitePool,
    id: &str,
    option_id: &str,
    value: Value,
) -> Result<(), Error> {
    if option_id.trim().is_empty() {
        return Err(Error::Message("option_id required".into()));
    }
    let (json,): (String,) = sqlx::query_as("SELECT config_json FROM threads WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("thread not found".into()))?;
    let mut map = match serde_json::from_str::<Value>(&json)? {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    map.insert(option_id.to_string(), value);
    sqlx::query("UPDATE threads SET config_json = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(Value::Object(map).to_string())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn attach_files(pool: &SqlitePool, id: &str, paths: &[String]) -> Result<(), Error> {
    if paths.is_empty() {
        return Err(Error::Message("no files to attach".into()));
    }
    let ctx = context(pool, id).await?;
    let (json,): (String,) = sqlx::query_as("SELECT config_json FROM threads WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let mut map = match serde_json::from_str::<Value>(&json)? {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let mut attached: Vec<String> = map
        .get("attachedFiles")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut extra = ctx.extra_roots;

    for path in paths {
        let given = Path::new(path);
        let candidate: PathBuf = if given.is_absolute() {
            given.to_path_buf()
        } else if let Some(folder) = ctx.workspace_folder.as_deref() {
            Path::new(folder).join(given)
        } else {
            return Err(Error::Message(format!("{path} is not an absolute path")));
        };
        if !candidate.exists() {
            return Err(Error::Message(format!("file not found: {path}")));
        }
        let canon = candidate.canonicalize()?;
        let canon_s = canon.to_string_lossy().into_owned();
        if !attached.iter().any(|item| item == &canon_s) {
            attached.push(canon_s);
        }
        let root = if canon.is_dir() {
            canon.clone()
        } else {
            canon
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| canon.clone())
        };
        let root_s = root.to_string_lossy().into_owned();
        if !extra.iter().any(|item| item == &root_s) {
            extra.push(root_s);
        }
    }

    map.insert("attachedFiles".into(), json!(attached));
    sqlx::query(
        "UPDATE threads SET extra_roots_json = ?1, config_json = ?2, updated_at = ?3 WHERE id = ?4",
    )
    .bind(serde_json::to_string(&extra)?)
    .bind(Value::Object(map).to_string())
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
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
    let title: String = prose
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(64)
        .collect();
    sqlx::query("UPDATE threads SET updated_at = ?1, unread = 0, title = CASE WHEN title = 'New thread' AND ?3 != '' THEN ?3 ELSE title END WHERE id = ?2")
        .bind(now())
        .bind(id)
        .bind(title)
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
    async fn history_is_ordered_and_scoped_to_one_thread_kind() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let a = create(&pool, None, "opencode".into()).await.unwrap();
        let b = create(&pool, None, "opencode".into()).await.unwrap();
        append_message(&pool, &a.id, "thread", "user", "first")
            .await
            .unwrap();
        append_message(&pool, &b.id, "thread", "user", "other thread")
            .await
            .unwrap();
        append_message(&pool, &a.id, "agent", "assistant", "other kind")
            .await
            .unwrap();
        append_message(&pool, &a.id, "thread", "assistant", "second")
            .await
            .unwrap();
        let lines = history(&pool, &a.id).await.unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(lines[1].role, "assistant");
        assert!(history(&pool, "missing").await.is_err());
    }

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
        assert_eq!(list(&pool, None).await.unwrap()[0].title, "hi");
        rename(&pool, &thread.id, "My chosen title").await.unwrap();
        send(
            &pool,
            &thread.id,
            &[ContentPart {
                r#type: "text".into(),
                text: Some("second turn".into()),
                path: None,
            }],
        )
        .await
        .unwrap();
        assert_eq!(list(&pool, None).await.unwrap()[0].title, "My chosen title");
        append_message(&pool, &thread.id, "thread", "assistant", "ok")
            .await
            .unwrap();
        set_acp_session(&pool, &thread.id, "sess-1").await.unwrap();
        let ctx = context(&pool, &thread.id).await.unwrap();
        assert_eq!(ctx.engine_id, "opencode");
        assert_eq!(ctx.acp_session.as_deref(), Some("sess-1"));
    }

    #[tokio::test]
    async fn set_config_merges_and_attach_files_persist() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let thread = create(&pool, None, "opencode".into()).await.unwrap();
        set_config(&pool, &thread.id, "model", json!("local"))
            .await
            .unwrap();
        set_config(&pool, &thread.id, "effort", json!("high"))
            .await
            .unwrap();
        let (config,): (String,) = sqlx::query_as("SELECT config_json FROM threads WHERE id = ?1")
            .bind(&thread.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(config.contains("model") && config.contains("effort"));

        let file = dir.path().join("note.txt");
        std::fs::write(&file, "hi").unwrap();
        attach_files(&pool, &thread.id, &[file.display().to_string()])
            .await
            .unwrap();
        let ctx = context(&pool, &thread.id).await.unwrap();
        assert!(ctx.extra_roots.iter().any(|root| Path::new(root).exists()));
        let (config,): (String,) = sqlx::query_as("SELECT config_json FROM threads WHERE id = ?1")
            .bind(&thread.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(config.contains("attachedFiles"));
        assert!(attach_files(&pool, &thread.id, &[]).await.is_err());
        assert!(
            attach_files(
                &pool,
                &thread.id,
                &["/definitely-missing-harbor-file".into()]
            )
            .await
            .is_err()
        );
    }
}
