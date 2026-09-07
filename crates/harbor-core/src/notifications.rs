//! The inbox behind the title-bar bell.
//!
//! Work runs in workspaces the builder is not looking at, so anything that
//! finishes, fails, or needs an answer is recorded here with enough of a target
//! to navigate back to it.

use serde_json::{Value, json};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::Error, types::Notification};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// Where a notification points. Every field is optional: a row is still useful
/// when Harbor only knows the mode it came from.
#[derive(Debug, Clone, Default)]
pub struct Target {
    pub mode: Option<String>,
    pub workspace_id: Option<String>,
    pub pane_id: Option<String>,
    pub session_ref: Option<String>,
}

impl Target {
    pub fn code(workspace_id: impl Into<String>, pane_id: impl Into<String>) -> Self {
        Self {
            mode: Some("code".into()),
            workspace_id: Some(workspace_id.into()),
            pane_id: Some(pane_id.into()),
            session_ref: None,
        }
    }

    pub fn session(mode: &str, session_ref: impl Into<String>) -> Self {
        Self {
            mode: Some(mode.into()),
            workspace_id: None,
            pane_id: None,
            session_ref: Some(session_ref.into()),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "mode": self.mode,
            "workspaceId": self.workspace_id,
            "paneId": self.pane_id,
            "sessionRef": self.session_ref,
        })
    }
}

/// Records one event. Returns the stored row so the host can emit it.
pub async fn record(
    pool: &SqlitePool,
    kind: &str,
    title: &str,
    body: &str,
    target: Target,
) -> Result<Notification, Error> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Message("title required".into()));
    }
    let id = Uuid::now_v7().to_string();
    let created_at = now();
    sqlx::query(
        "INSERT INTO notifications (id, kind, title, body, target_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(kind)
    .bind(title)
    .bind(body.trim())
    .bind(target.to_json().to_string())
    .bind(created_at)
    .execute(pool)
    .await?;
    Ok(Notification {
        id,
        kind: kind.to_string(),
        title: title.to_string(),
        body: body.trim().to_string(),
        read: false,
        created_at,
        mode: target.mode,
        workspace_id: target.workspace_id,
        pane_id: target.pane_id,
        session_ref: target.session_ref,
    })
}

fn row_to_notification(
    id: String,
    kind: String,
    title: String,
    body: String,
    target_json: String,
    read_at: Option<i64>,
    created_at: i64,
) -> Notification {
    let target: Value = serde_json::from_str(&target_json).unwrap_or(Value::Null);
    let field = |name: &str| target.get(name).and_then(Value::as_str).map(str::to_string);
    Notification {
        id,
        kind,
        title,
        body,
        read: read_at.is_some(),
        created_at,
        mode: field("mode"),
        workspace_id: field("workspaceId"),
        pane_id: field("paneId"),
        session_ref: field("sessionRef"),
    }
}

/// id, kind, title, body, target_json, read_at, created_at
type Row = (String, String, String, String, String, Option<i64>, i64);

/// Newest first, capped: the bell is a recent-events list, not an archive.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Notification>, Error> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, kind, title, body, target_json, read_at, created_at
         FROM notifications ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, kind, title, body, target_json, read_at, created_at)| {
                row_to_notification(id, kind, title, body, target_json, read_at, created_at)
            },
        )
        .collect())
}

pub async fn unread_count(pool: &SqlitePool) -> Result<i64, Error> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE read_at IS NULL")
            .fetch_one(pool)
            .await?;
    Ok(count)
}

pub async fn mark_read(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query("UPDATE notifications SET read_at = ?1 WHERE read_at IS NULL")
        .bind(now())
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn records_targets_and_counts_unread() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        assert_eq!(unread_count(&pool).await.unwrap(), 0);

        record(
            &pool,
            "terminal-exit",
            "Claude Code stopped",
            "exit code 1",
            Target::code("ws-1", "term-2"),
        )
        .await
        .unwrap();
        record(
            &pool,
            "permission",
            "Codex needs permission",
            "Write src/main.rs",
            Target::session("chat", "thread-9"),
        )
        .await
        .unwrap();

        assert_eq!(unread_count(&pool).await.unwrap(), 2);
        let rows = list(&pool).await.unwrap();
        assert_eq!(rows.len(), 2);

        // Newest first, and each keeps the target it can navigate back to.
        assert_eq!(rows[0].kind, "permission");
        assert_eq!(rows[0].mode.as_deref(), Some("chat"));
        assert_eq!(rows[0].session_ref.as_deref(), Some("thread-9"));
        assert_eq!(rows[0].pane_id, None);

        assert_eq!(rows[1].kind, "terminal-exit");
        assert_eq!(rows[1].workspace_id.as_deref(), Some("ws-1"));
        assert_eq!(rows[1].pane_id.as_deref(), Some("term-2"));
        assert!(rows.iter().all(|row| !row.read));

        mark_read(&pool).await.unwrap();
        assert_eq!(unread_count(&pool).await.unwrap(), 0);
        assert!(list(&pool).await.unwrap().iter().all(|row| row.read));
    }

    #[tokio::test]
    async fn a_row_without_a_title_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        assert!(
            record(&pool, "test", "   ", "body", Target::default())
                .await
                .is_err()
        );
        assert_eq!(unread_count(&pool).await.unwrap(), 0);
    }
}
