use sqlx::SqlitePool;

use crate::{error::Error, types::SearchHit};

pub async fn session_search(
    pool: &SqlitePool,
    agent_id: &str,
    query: &str,
) -> Result<Vec<SearchHit>, Error> {
    let Some(match_query) = fts5_match(query) else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT m.chat_id, m.prose, m.created_at
         FROM messages_fts f
         JOIN messages m ON m.rowid = f.rowid
         JOIN agent_chats c ON c.id = m.chat_id
         WHERE messages_fts MATCH ?1
           AND m.chat_kind = 'agent'
           AND c.agent_id = ?2
           AND m.role IN ('user', 'assistant')
         ORDER BY m.created_at DESC
         LIMIT 50",
    )
    .bind(match_query)
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(chat_id, prose, created_at)| SearchHit {
            chat_id,
            prose,
            created_at,
        })
        .collect())
}

fn fts5_match(query: &str) -> Option<String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .filter_map(|token| {
            let cleaned: String = token
                .chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '-')
                .collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(format!("\"{cleaned}\""))
            }
        })
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::{ContentPart, CreateAgent};

    fn text(body: &str) -> ContentPart {
        ContentPart {
            r#type: "text".into(),
            text: Some(body.into()),
            path: None,
        }
    }

    #[tokio::test]
    async fn searches_agent_prose_not_tool_output() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let ada = crate::agents::create(
            &pool,
            CreateAgent {
                name: "Ada".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        let other = crate::agents::create(
            &pool,
            CreateAgent {
                name: "Other".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        let chat = crate::chats::create(&pool, &ada.id).await.unwrap();
        let other_chat = crate::chats::create(&pool, &other.id).await.unwrap();
        crate::chats::send(&pool, &chat.id, &[text("alpha rust backend")])
            .await
            .unwrap();
        crate::threads::append_message(&pool, &chat.id, "agent", "assistant", "later rust reply")
            .await
            .unwrap();
        crate::threads::append_message(&pool, &chat.id, "agent", "tool", "rust tool dump")
            .await
            .unwrap();
        crate::chats::send(&pool, &other_chat.id, &[text("rust elsewhere")])
            .await
            .unwrap();

        let hits = session_search(&pool, &ada.id, "rust").await.unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|hit| hit.chat_id == chat.id));
        assert!(hits.iter().all(|hit| !hit.prose.contains("tool dump")));
        assert!(hits[0].prose.contains("later") || hits[0].prose.contains("alpha"));
        assert!(
            session_search(&pool, &ada.id, "   ")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            session_search(&pool, &ada.id, "nomatchtoken")
                .await
                .unwrap()
                .is_empty()
        );
    }
}
