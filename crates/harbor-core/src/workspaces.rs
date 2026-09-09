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

/// A display name for the rail. Blank resets to the folder's own name, so a
/// builder can always get the default back without re-adding the folder.
pub async fn rename(pool: &SqlitePool, id: &str, title: &str) -> Result<Workspace, Error> {
    let folder: Option<(String,)> = sqlx::query_as("SELECT folder FROM workspaces WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let (folder,) = folder.ok_or_else(|| Error::Message("workspace not found".into()))?;

    let trimmed = title.trim();
    let title = if trimmed.is_empty() {
        std::path::Path::new(&folder)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Workspace")
            .to_string()
    } else {
        if trimmed.chars().count() > 60 {
            return Err(Error::Message("Use 60 characters or fewer.".into()));
        }
        trimmed.to_string()
    };

    let (id, folder, title, pinned) = sqlx::query_as::<_, (String, String, Option<String>, i64)>(
        "UPDATE workspaces SET title = ?1 WHERE id = ?2
         RETURNING id, folder, title, pinned",
    )
    .bind(&title)
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(Workspace {
        id,
        folder,
        title,
        pinned: pinned != 0,
    })
}

/// Tabs and panes cascade, but `threads.workspace_id` carries no foreign key,
/// so the folder's threads would be stranded on an id nothing lists any more.
/// They move to Other instead: the folder leaves the rail, the conversations
/// stay reachable.
pub async fn remove(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE threads SET workspace_id = NULL WHERE workspace_id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM workspaces WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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

    #[tokio::test]
    async fn remove_moves_the_folders_threads_to_other() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("state/db.sqlite"))
            .await
            .unwrap();
        let folder = dir.path().join("project");
        std::fs::create_dir(&folder).unwrap();
        let workspace = add(&pool, folder.display().to_string()).await.unwrap();
        let thread = crate::threads::create(&pool, Some(workspace.id.clone()), "opencode".into())
            .await
            .unwrap();

        remove(&pool, &workspace.id).await.unwrap();

        assert!(list(&pool).await.unwrap().is_empty());
        let orphaned = crate::threads::list(&pool, None).await.unwrap();
        assert_eq!(orphaned.len(), 1);
        assert_eq!(orphaned[0].id, thread.id);
        assert_eq!(orphaned[0].workspace_id, None);
    }

    #[tokio::test]
    async fn rename_trims_resets_on_blank_and_rejects_long_titles() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("state/db.sqlite"))
            .await
            .unwrap();
        let folder = dir.path().join("Free Project");
        std::fs::create_dir_all(&folder).unwrap();
        let added = add(&pool, folder.to_string_lossy().into_owned())
            .await
            .unwrap();
        assert_eq!(added.title.as_deref(), Some("Free Project"));

        let renamed = rename(&pool, &added.id, "  Launch work  ").await.unwrap();
        assert_eq!(renamed.title.as_deref(), Some("Launch work"));
        assert_eq!(renamed.id, added.id);
        assert_eq!(
            list(&pool).await.unwrap()[0].title.as_deref(),
            Some("Launch work")
        );

        // Blank restores the folder's own name rather than leaving an empty rail row.
        let reset = rename(&pool, &added.id, "   ").await.unwrap();
        assert_eq!(reset.title.as_deref(), Some("Free Project"));

        assert!(rename(&pool, &added.id, &"x".repeat(61)).await.is_err());
        assert!(rename(&pool, "missing", "Nope").await.is_err());
    }
}
