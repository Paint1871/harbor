use sqlx::SqlitePool;

use crate::{error::Error, types::SearchHit};

/// Full-text search over user/assistant prose. `agent_id` scopes to one agent's
/// chats; `workspace_id` scopes to a folder's threads (`""` selects threads
/// that belong to no workspace). At least one scope must be non-empty — the
/// caller always knows which surface it is searching.
pub async fn session_search(
    pool: &SqlitePool,
    agent_id: Option<&str>,
    workspace_id: Option<&str>,
    query: &str,
) -> Result<Vec<SearchHit>, Error> {
    let Some(match_query) = fts5_match(query) else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();

    if let Some(agent_id) = agent_id.filter(|id| !id.is_empty()) {
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
        .bind(&match_query)
        .bind(agent_id)
        .fetch_all(pool)
        .await?;
        hits.extend(
            rows.into_iter()
                .map(|(chat_id, prose, created_at)| SearchHit {
                    chat_id,
                    chat_kind: "agent".into(),
                    prose,
                    created_at,
                }),
        );
    }

    if let Some(workspace_id) = workspace_id {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT m.chat_id, m.prose, m.created_at
             FROM messages_fts f
             JOIN messages m ON m.rowid = f.rowid
             JOIN threads t ON t.id = m.chat_id
             WHERE messages_fts MATCH ?1
               AND m.chat_kind = 'thread'
               AND (?2 = '' AND t.workspace_id IS NULL OR t.workspace_id = ?2)
               AND m.role IN ('user', 'assistant')
             ORDER BY m.created_at DESC
             LIMIT 50",
        )
        .bind(&match_query)
        .bind(workspace_id)
        .fetch_all(pool)
        .await?;
        hits.extend(
            rows.into_iter()
                .map(|(chat_id, prose, created_at)| SearchHit {
                    chat_id,
                    chat_kind: "thread".into(),
                    prose,
                    created_at,
                }),
        );
    }

    hits.sort_by_key(|hit| std::cmp::Reverse(hit.created_at));
    hits.truncate(50);
    Ok(hits)
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
                face_index: None,
                home_path: None,
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
                face_index: None,
                home_path: None,
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

        let hits = session_search(&pool, Some(&ada.id), None, "rust")
            .await
            .unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|hit| hit.chat_id == chat.id));
        assert!(hits.iter().all(|hit| hit.chat_kind == "agent"));
        assert!(hits.iter().all(|hit| !hit.prose.contains("tool dump")));
        assert!(hits[0].prose.contains("later") || hits[0].prose.contains("alpha"));
        assert!(
            session_search(&pool, Some(&ada.id), None, "   ")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            session_search(&pool, Some(&ada.id), None, "nomatchtoken")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn workspace_scope_searches_thread_prose() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        std::fs::create_dir(dir.path().join("ws")).unwrap();
        std::fs::create_dir(dir.path().join("other")).unwrap();
        let workspace = crate::workspaces::add(&pool, dir.path().join("ws").display().to_string())
            .await
            .unwrap();
        let other_workspace =
            crate::workspaces::add(&pool, dir.path().join("other").display().to_string())
                .await
                .unwrap();
        let thread = crate::threads::create(&pool, Some(workspace.id.clone()), "opencode".into())
            .await
            .unwrap();
        let elsewhere =
            crate::threads::create(&pool, Some(other_workspace.id.clone()), "opencode".into())
                .await
                .unwrap();
        let loose = crate::threads::create(&pool, None, "opencode".into())
            .await
            .unwrap();
        crate::threads::append_message(&pool, &thread.id, "thread", "user", "rust in this folder")
            .await
            .unwrap();
        crate::threads::append_message(&pool, &elsewhere.id, "thread", "user", "rust elsewhere")
            .await
            .unwrap();
        crate::threads::append_message(&pool, &loose.id, "thread", "user", "rust without a folder")
            .await
            .unwrap();

        let hits = session_search(&pool, None, Some(&workspace.id), "rust")
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chat_id, thread.id);
        assert_eq!(hits[0].chat_kind, "thread");

        // The empty scope id selects threads that belong to no workspace.
        let hits = session_search(&pool, None, Some(""), "rust").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chat_id, loose.id);

        // No scope at all never fabricates a result.
        assert!(
            session_search(&pool, None, None, "rust")
                .await
                .unwrap()
                .is_empty()
        );
    }
}
