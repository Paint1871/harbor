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
    let path = std::path::Path::new(folder.trim());
    if !path.is_absolute() || !path.is_dir() {
        return Err(Error::Message(
            "Choose an existing folder with an absolute path.".into(),
        ));
    }
    let folder = path.canonicalize()?.to_string_lossy().into_owned();
    let id = Uuid::now_v7().to_string();
    let title = std::path::Path::new(&folder)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Workspace")
        .to_string();
    let (id, folder, title, pinned) = sqlx::query_as::<_, (String, String, Option<String>, i64)>(
        "INSERT INTO workspaces (id, folder, title, pinned, last_opened) VALUES (?1, ?2, ?3, 0, ?4)
         ON CONFLICT(folder) DO UPDATE SET last_opened = excluded.last_opened
         RETURNING id, folder, title, pinned",
    )
    .bind(&id)
    .bind(&folder)
    .bind(&title)
    .bind(now())
    .fetch_one(pool)
    .await?;
    crate::layout::ensure_default_tab(pool, &id).await?;
    Ok(Workspace {
        id,
        folder,
        title,
        pinned: pinned != 0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reopening_folder_preserves_id_and_pin_and_rejects_invalid_paths() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("state/db.sqlite"))
            .await
            .unwrap();
        let project = dir.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let first = add(&pool, project.display().to_string()).await.unwrap();
        pin(&pool, &first.id, true).await.unwrap();
        let again = add(&pool, project.join(".").display().to_string())
            .await
            .unwrap();
        assert_eq!(first.id, again.id);
        assert!(again.pinned);
        assert_eq!(list(&pool).await.unwrap().len(), 1);
        assert!(add(&pool, "relative/path".into()).await.is_err());
        assert!(
            add(&pool, project.join("missing").display().to_string())
                .await
                .is_err()
        );
        assert!(
            add(
                &pool,
                dir.path().join("state/db.sqlite").display().to_string()
            )
            .await
            .is_err()
        );
    }
}
