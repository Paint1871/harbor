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

    crate::notifications::record(
        pool,
        "mail",
        &format!("Mail from {from_name}"),
        body,
        crate::notifications::Target {
            mode: Some("agent".into()),
            workspace_id: None,
            pane_id: None,
            session_ref: Some(chat.id.clone()),
        },
    )
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
    async fn mail_notifies_the_recipient_with_a_target_that_opens_the_chat() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        assert!(crate::notifications::list(&pool).await.unwrap().is_empty());

        let from = agent(&pool, "Scout").await;
        let to = agent(&pool, "Release manager").await;
        send(&pool, &from, &to, "handoff please").await.unwrap();

        let rows = crate::notifications::list(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "mail");
        assert_eq!(rows[0].title, "Mail from Scout");
        assert_eq!(rows[0].body, "handoff please");
        // The row points at the recipient's Mail chat, not just at Agent mode.
        assert_eq!(rows[0].mode.as_deref(), Some("agent"));
        let chats = crate::chats::list(&pool, &to).await.unwrap();
        assert_eq!(rows[0].session_ref.as_deref(), Some(chats[0].id.as_str()));
    }
}
