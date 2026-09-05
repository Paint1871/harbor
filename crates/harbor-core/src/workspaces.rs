use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::Error, types::Workspace};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Workspace>, Error> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, i64)>(
        "SELECT id, folder, title, pinned FROM workspaces ORDER BY pinned DESC, last_opened DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, folder, title, pinned)| Workspace {
            id,
            folder,
            title,
            pinned: pinned != 0,
        })
        .collect())
}

pub async fn add(pool: &SqlitePool, folder: String) -> Result<Workspace, Error> {
    let folder = folder.trim().to_string();
    if folder.is_empty() {
        return Err(Error::Message("folder required".into()));
    }
    let id = Uuid::now_v7().to_string();
    let title = std::path::Path::new(&folder)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Workspace")
        .to_string();
    sqlx::query(
        "INSERT INTO workspaces (id, folder, title, pinned, last_opened) VALUES (?1, ?2, ?3, 0, ?4)",
    )
    .bind(&id)
    .bind(&folder)
    .bind(&title)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(Workspace {
        id,
        folder,
        title: Some(title),
        pinned: false,
    })
}

pub async fn remove(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    sqlx::query("DELETE FROM workspaces WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn pin(pool: &SqlitePool, id: &str, pinned: bool) -> Result<(), Error> {
    sqlx::query("UPDATE workspaces SET pinned = ?1 WHERE id = ?2")
        .bind(i64::from(pinned))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
