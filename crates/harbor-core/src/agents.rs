use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::Error,
    types::{AgentRecord, CreateAgent, UpdateAgent},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<AgentRecord>, Error> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64)>(
        "SELECT id, name, brief, engine_id, face_index, pinned FROM agents ORDER BY pinned DESC, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, name, brief, engine_id, face_index, pinned)| AgentRecord {
                id,
                name,
                brief,
                engine_id,
                face_index: face_index as i32,
                pinned: pinned != 0,
            },
        )
        .collect())
}

pub async fn create(pool: &SqlitePool, input: CreateAgent) -> Result<AgentRecord, Error> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(Error::Message("name required".into()));
    }
    let id = Uuid::now_v7().to_string();
    let ts = now();
    sqlx::query(
        "INSERT INTO agents (id, name, brief, engine_id, face_index, home_path, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&input.brief)
    .bind(&input.engine_id)
    .bind(input.face_index)
    .bind("")
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(AgentRecord {
        id,
        name,
        brief: input.brief,
        engine_id: input.engine_id,
        face_index: input.face_index,
        pinned: false,
    })
}

pub async fn update(pool: &SqlitePool, input: UpdateAgent) -> Result<(), Error> {
    if let Some(name) = input.name.as_ref() {
        sqlx::query("UPDATE agents SET name = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(name)
            .bind(now())
            .bind(&input.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM agents WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
