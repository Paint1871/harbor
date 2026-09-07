use std::path::Path;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{agents, error::Error, types::Place};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub fn canonicalize_folder(path: &str) -> Result<String, Error> {
    let given = Path::new(path.trim());
    if !given.is_absolute() || !given.is_dir() {
        return Err(Error::Message(
            "Choose an existing folder with an absolute path.".into(),
        ));
    }
    Ok(given.canonicalize()?.to_string_lossy().into_owned())
}

pub async fn granted_paths(pool: &SqlitePool) -> Result<Vec<String>, Error> {
    let homes =
        sqlx::query_as::<_, (String,)>("SELECT home_path FROM agents WHERE home_path != ''")
            .fetch_all(pool)
            .await?;
    let extras = sqlx::query_as::<_, (String,)>("SELECT path FROM places")
        .fetch_all(pool)
        .await?;
    let mut paths: Vec<String> = homes.into_iter().map(|(path,)| path).collect();
    for (path,) in extras {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub async fn grant(pool: &SqlitePool, agent_id: &str, path: &str) -> Result<(), Error> {
    agents::require(pool, agent_id).await?;
    let folder = canonicalize_folder(path)?;
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM places WHERE agent_id = ?1 AND path = ?2")
            .bind(agent_id)
            .bind(&folder)
            .fetch_optional(pool)
            .await?;
    if existing.is_some() {
        return Ok(());
    }
    let id = Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO places (id, agent_id, path, granted_at) VALUES (?1, ?2, ?3, ?4)")
        .bind(&id)
        .bind(agent_id)
        .bind(&folder)
        .bind(now())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    let result = sqlx::query("DELETE FROM places WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("place not found".into()));
    }
    Ok(())
}

pub async fn list(pool: &SqlitePool, agent_id: &str) -> Result<Vec<Place>, Error> {
    agents::require(pool, agent_id).await?;
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, path FROM places WHERE agent_id = ?1 ORDER BY granted_at, rowid",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, path)| Place { id, path })
        .collect())
}

pub async fn paths_for(pool: &SqlitePool, agent_id: &str) -> Result<Vec<String>, Error> {
    Ok(list(pool, agent_id)
        .await?
        .into_iter()
        .map(|place| place.path)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::CreateAgent;

    #[tokio::test]
    async fn grant_canonicalizes_and_skips_dupes() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let folder = dir.path().join("proj");
        std::fs::create_dir(&folder).unwrap();
        let agent = crate::agents::create(
            &pool,
            CreateAgent {
                name: "Places".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        grant(&pool, &agent.id, &folder.display().to_string())
            .await
            .unwrap();
        grant(&pool, &agent.id, &folder.join(".").display().to_string())
            .await
            .unwrap();
        let granted = list(&pool, &agent.id).await.unwrap();
        assert_eq!(granted.len(), 1);
        assert!(granted[0].path.ends_with("proj") || granted[0].path.contains("proj"));
        let all = crate::places::granted_paths(&pool).await.unwrap();
        assert!(
            all.iter()
                .any(|path| path.ends_with("proj") || path.contains("proj"))
        );
        assert!(!granted[0].id.is_empty());
        assert_eq!(
            crate::commands::places_list(&pool, agent.id.clone())
                .await
                .unwrap()
                .len(),
            1
        );

        assert!(grant(&pool, &agent.id, "relative").await.is_err());
        assert!(
            grant(
                &pool,
                &agent.id,
                &dir.path().join("missing").display().to_string()
            )
            .await
            .is_err()
        );

        revoke(&pool, &granted[0].id).await.unwrap();
        assert!(list(&pool, &agent.id).await.unwrap().is_empty());
        assert!(revoke(&pool, &granted[0].id).await.is_err());
        assert!(list(&pool, "missing").await.is_err());
        assert!(
            crate::places::granted_paths(&pool)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
