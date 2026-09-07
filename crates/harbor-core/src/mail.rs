use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{chats, error::Error};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn send(
    pool: &SqlitePool,
    from_agent_id: &str,
    to_agent_id: &str,
    body: &str,
) -> Result<(), Error> {
    let body = body.trim();
    if body.is_empty() {
        return Err(Error::Message("body required".into()));
    }
    let from: Option<(String, String, i64)> =
        sqlx::query_as("SELECT id, name, messaging FROM agents WHERE id = ?1")
            .bind(from_agent_id)
            .fetch_optional(pool)
            .await?;
    let to: Option<(String, String, i64)> =
        sqlx::query_as("SELECT id, name, messaging FROM agents WHERE id = ?1")
            .bind(to_agent_id)
            .fetch_optional(pool)
            .await?;
    let (from_id, from_name, _from_messaging) =
        from.ok_or_else(|| Error::Message("sender not found".into()))?;
    let (to_id, _to_name, to_messaging) =
        to.ok_or_else(|| Error::Message("recipient not found".into()))?;

    let paused = crate::settings::get(pool, "messaging_paused")
        .await?
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if paused && to_messaging == 0 {
        return Err(Error::Message("Messaging is paused for this agent".into()));
    }

    let chat = chats::find_or_create_titled(pool, &to_id, "Mail").await?;
    let payload = json!({
        "fromAgentId": from_id,
        "toAgentId": to_id,
    });
    let message_id = Uuid::now_v7().to_string();
    let ts = now();
    sqlx::query(
        "INSERT INTO messages (id, chat_id, chat_kind, role, prose, payload_json, created_at)
         VALUES (?1, ?2, 'agent', 'mail', ?3, ?4, ?5)",
    )
    .bind(&message_id)
    .bind(&chat.id)
    .bind(body)
    .bind(payload.to_string())
    .bind(ts)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE agent_chats SET updated_at = ?1 WHERE id = ?2")
        .bind(ts)
        .bind(&chat.id)
        .execute(pool)
        .await?;

    let notification_id = Uuid::now_v7().to_string();
    let target = json!({
        "mode": "agent",
        "agentId": to_id,
        "chatId": chat.id,
    });
    sqlx::query(
        "INSERT INTO notifications (id, kind, title, body, target_json, created_at)
         VALUES (?1, 'mail', ?2, ?3, ?4, ?5)",
    )
    .bind(&notification_id)
    .bind(format!("Mail from {from_name}"))
    .bind(body)
    .bind(target.to_string())
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

/// Newest first. The Inbox shows only what the local database actually holds.
pub async fn notifications(pool: &SqlitePool) -> Result<Vec<crate::types::Notification>, Error> {
    let rows: Vec<(String, String, String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT id, kind, title, body, read_at, created_at
         FROM notifications ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, kind, title, body, read_at, created_at)| crate::types::Notification {
                id,
                kind,
                title,
                body,
                read: read_at.is_some(),
                created_at,
            },
        )
        .collect())
}

pub async fn mark_notifications_read(pool: &SqlitePool) -> Result<(), Error> {
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
    use crate::types::CreateAgent;
    use serde_json::json;

    async fn agent(pool: &SqlitePool, name: &str) -> String {
        crate::agents::create(
            pool,
            CreateAgent {
                name: name.into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn send_writes_mail_chat_and_notification() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let from = agent(&pool, "From").await;
        let to = agent(&pool, "To").await;
        send(&pool, &from, &to, "handoff please").await.unwrap();
        send(&pool, &from, &to, "second note").await.unwrap();
        let chats = crate::chats::list(&pool, &to).await.unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title, "Mail");
        let history = crate::chats::history(&pool, &chats[0].id).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "mail");
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);

        crate::settings::set(&pool, "messaging_paused", &json!(true))
            .await
            .unwrap();
        let paused = send(&pool, &from, &to, "blocked").await.unwrap_err();
        assert_eq!(paused.to_string(), "Messaging is paused for this agent");
        assert!(send(&pool, &from, "missing", "x").await.is_err());
        assert!(send(&pool, &from, &to, "  ").await.is_err());
    }

    #[tokio::test]
    async fn notifications_list_is_newest_first_and_marks_read() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        assert!(notifications(&pool).await.unwrap().is_empty());

        let from = agent(&pool, "From").await;
        let to = agent(&pool, "To").await;
        send(&pool, &from, &to, "first").await.unwrap();
        send(&pool, &from, &to, "second").await.unwrap();

        let rows = notifications(&pool).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].body, "second");
        assert_eq!(rows[0].kind, "mail");
        assert!(rows.iter().all(|row| !row.read));

        mark_notifications_read(&pool).await.unwrap();
        assert!(
            notifications(&pool)
                .await
                .unwrap()
                .iter()
                .all(|row| row.read)
        );
    }
}
