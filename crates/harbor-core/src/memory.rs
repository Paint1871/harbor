use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{agents, error::Error, types::Memory};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn list(pool: &SqlitePool, agent_id: &str) -> Result<Vec<Memory>, Error> {
    agents::require(pool, agent_id).await?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, body, kind FROM memories WHERE agent_id = ?1 ORDER BY created_at, rowid",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, body, kind)| Memory { id, body, kind })
        .collect())
}

pub async fn upsert(pool: &SqlitePool, agent_id: &str, body: &str) -> Result<Memory, Error> {
    let body = body.trim();
    if body.is_empty() {
        return Err(Error::Message("body required".into()));
    }
    agents::require(pool, agent_id).await?;
    let id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO memories (id, agent_id, body, kind, created_at) VALUES (?1, ?2, ?3, 'fact', ?4)",
    )
    .bind(&id)
    .bind(agent_id)
    .bind(body)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(Memory {
        id,
        body: body.into(),
        kind: "fact".into(),
    })
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    let result = sqlx::query("DELETE FROM memories WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("memory not found".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::CreateAgent;

    #[tokio::test]
    async fn upsert_list_delete_and_reject_empty() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let agent = crate::agents::create(
            &pool,
            CreateAgent {
                name: "Mem".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        let err = upsert(&pool, &agent.id, "  ").await.unwrap_err();
        assert_eq!(err.to_string(), "body required");
        let first = upsert(&pool, &agent.id, "Prefers tests").await.unwrap();
        let second = upsert(&pool, &agent.id, "No secrets").await.unwrap();
        let items = list(&pool, &agent.id).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, first.id);
        assert_eq!(items[1].body, "No secrets");
        delete(&pool, &second.id).await.unwrap();
        assert_eq!(list(&pool, &agent.id).await.unwrap().len(), 1);
        assert!(delete(&pool, "missing").await.is_err());
    }
}
