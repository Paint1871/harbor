use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::Error, types::ThreadRecord};

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
