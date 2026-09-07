use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    agents,
    error::Error,
    threads,
    types::{AgentChat, ChatMessage, ContentPart},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct ChatContext {
    pub id: String,
    pub agent_id: String,
    pub engine_id: String,
    pub home_path: String,
    pub extra_dirs: Vec<String>,
    pub acp_session: Option<String>,
}

type ChatRow = (String, String, String, String);

fn map_chat((id, agent_id, title, status): ChatRow) -> AgentChat {
    AgentChat {
        id,
        agent_id,
        title,
        status,
    }
}

pub async fn list(pool: &SqlitePool, agent_id: &str) -> Result<Vec<AgentChat>, Error> {
    agents::require(pool, agent_id).await?;
    let rows = sqlx::query_as::<_, ChatRow>(
        "SELECT id, agent_id, title, status FROM agent_chats
         WHERE agent_id = ?1 ORDER BY tab_order, created_at",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(map_chat).collect())
}

pub async fn create(pool: &SqlitePool, agent_id: &str) -> Result<AgentChat, Error> {
    agents::require(pool, agent_id).await?;
    let (tab_order,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(tab_order), -1) + 1 FROM agent_chats WHERE agent_id = ?1",
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await?;
    let id = Uuid::now_v7().to_string();
    let ts = now();
    sqlx::query(
        "INSERT INTO agent_chats (id, agent_id, title, status, tab_order, config_json, created_at, updated_at)
         VALUES (?1, ?2, 'New chat', 'idle', ?3, '{}', ?4, ?4)",
    )
    .bind(&id)
    .bind(agent_id)
    .bind(tab_order)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(AgentChat {
        id,
        agent_id: agent_id.into(),
        title: "New chat".into(),
        status: "idle".into(),
    })
}

pub async fn send(pool: &SqlitePool, chat_id: &str, parts: &[ContentPart]) -> Result<(), Error> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM agent_chats WHERE id = ?1")
        .bind(chat_id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(Error::Message("chat not found".into()));
    }
    let prose = parts
        .iter()
        .filter_map(|part| part.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    threads::append_message(pool, chat_id, "agent", "user", &prose).await?;
    let title: String = prose
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(64)
        .collect();
    sqlx::query(
        "UPDATE agent_chats SET updated_at = ?1,
            title = CASE WHEN title = 'New chat' AND ?3 != '' THEN ?3 ELSE title END
         WHERE id = ?2",
    )
    .bind(now())
    .bind(chat_id)
    .bind(title)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn history(pool: &SqlitePool, chat_id: &str) -> Result<Vec<ChatMessage>, Error> {
    context(pool, chat_id).await?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, role, prose FROM messages
         WHERE chat_kind = 'agent' AND chat_id = ?1
         ORDER BY created_at, rowid",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, role, text)| ChatMessage { id, role, text })
        .collect())
}

type ContextRow = (String, String, String, String, Option<String>);

pub async fn context(pool: &SqlitePool, chat_id: &str) -> Result<ChatContext, Error> {
    let row: Option<ContextRow> = sqlx::query_as(
        "SELECT c.id, c.agent_id, a.engine_id, a.home_path, c.acp_session
         FROM agent_chats c
         JOIN agents a ON a.id = c.agent_id
         WHERE c.id = ?1",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;
    let (id, agent_id, engine_id, home_path, acp_session) =
        row.ok_or_else(|| Error::Message("chat not found".into()))?;
    let extra_dirs = crate::places::paths_for(pool, &agent_id).await?;
    Ok(ChatContext {
        id,
        agent_id,
        engine_id,
        home_path,
        extra_dirs,
        acp_session,
    })
}

pub async fn set_acp_session(pool: &SqlitePool, chat_id: &str, session: &str) -> Result<(), Error> {
    let result =
        sqlx::query("UPDATE agent_chats SET acp_session = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(session)
            .bind(now())
            .bind(chat_id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("chat not found".into()));
    }
    Ok(())
}

pub async fn set_config(
    pool: &SqlitePool,
    chat_id: &str,
    option_id: &str,
    value: Value,
) -> Result<(), Error> {
    if option_id.trim().is_empty() {
        return Err(Error::Message("option_id required".into()));
    }
    let (json,): (String,) = sqlx::query_as("SELECT config_json FROM agent_chats WHERE id = ?1")
        .bind(chat_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("chat not found".into()))?;
    let merged = merge_object(&json, option_id, value)?;
    sqlx::query("UPDATE agent_chats SET config_json = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(merged)
        .bind(now())
        .bind(chat_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_status(pool: &SqlitePool, chat_id: &str, status: &str) -> Result<(), Error> {
    let result = sqlx::query("UPDATE agent_chats SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status)
        .bind(now())
        .bind(chat_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("chat not found".into()));
    }
    Ok(())
}

pub async fn cancel(pool: &SqlitePool, chat_id: &str) -> Result<(), Error> {
    set_status(pool, chat_id, "idle").await
}

pub(crate) async fn find_or_create_titled(
    pool: &SqlitePool,
    agent_id: &str,
    title: &str,
) -> Result<AgentChat, Error> {
    let existing: Option<ChatRow> = sqlx::query_as(
        "SELECT id, agent_id, title, status FROM agent_chats
         WHERE agent_id = ?1 AND title = ?2
         ORDER BY tab_order, created_at LIMIT 1",
    )
    .bind(agent_id)
    .bind(title)
    .fetch_optional(pool)
    .await?;
    if let Some(row) = existing {
        return Ok(map_chat(row));
    }
    let chat = create(pool, agent_id).await?;
    sqlx::query("UPDATE agent_chats SET title = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(title)
        .bind(now())
        .bind(&chat.id)
        .execute(pool)
        .await?;
    Ok(AgentChat {
        title: title.into(),
        ..chat
    })
}

fn merge_object(json: &str, key: &str, value: Value) -> Result<String, Error> {
    let mut map = match serde_json::from_str::<Value>(json)? {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    map.insert(key.to_string(), value);
    Ok(Value::Object(map).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::CreateAgent;

    async fn agent(pool: &SqlitePool, name: &str) -> String {
        crate::agents::create(
            pool,
            CreateAgent {
                name: name.into(),
                brief: "b".into(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap()
        .id
    }

    fn text(body: &str) -> ContentPart {
        ContentPart {
            r#type: "text".into(),
            text: Some(body.into()),
            path: None,
        }
    }

    #[tokio::test]
    async fn create_send_history_config_and_cascade() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let agent_id = agent(&pool, "Ada").await;
        let first = create(&pool, &agent_id).await.unwrap();
        let second = create(&pool, &agent_id).await.unwrap();
        assert_eq!(first.title, "New chat");
        assert_eq!(first.status, "idle");
        assert_eq!(list(&pool, &agent_id).await.unwrap().len(), 2);

        send(&pool, &first.id, &[text("hello world")])
            .await
            .unwrap();
        send(&pool, &first.id, &[text("second turn")])
            .await
            .unwrap();
        let listed = list(&pool, &agent_id).await.unwrap();
        assert_eq!(listed[0].title, "hello world");
        let lines = crate::commands::agent_chat_history(&pool, first.id.clone())
            .await
            .unwrap();
        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec!["hello world", "second turn"]
        );

        set_config(&pool, &first.id, "model", Value::String("local".into()))
            .await
            .unwrap();
        let (json,): (String,) =
            sqlx::query_as("SELECT config_json FROM agent_chats WHERE id = ?1")
                .bind(&first.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(json.contains("model"));
        assert!(json.contains("local"));

        set_acp_session(&pool, &first.id, "sess-9").await.unwrap();
        let ctx = context(&pool, &first.id).await.unwrap();
        assert_eq!(ctx.agent_id, agent_id);
        assert_eq!(ctx.engine_id, "opencode");
        assert_eq!(ctx.acp_session.as_deref(), Some("sess-9"));

        cancel(&pool, &first.id).await.unwrap();
        assert_eq!(list(&pool, &agent_id).await.unwrap()[0].status, "idle");
        assert_eq!(list(&pool, &agent_id).await.unwrap()[1].id, second.id);

        crate::agents::delete(&pool, &agent_id).await.unwrap();
        let (chats,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_chats")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(chats, 0);
        let (messages,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(messages, 0);
    }
}
