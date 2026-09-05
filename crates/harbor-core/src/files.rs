use std::fs;
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::error::Error;
use crate::types::FsEntry;

async fn workspace_folder(pool: &SqlitePool, workspace_id: &str) -> Result<PathBuf, Error> {
    let (folder,): (String,) = sqlx::query_as("SELECT folder FROM workspaces WHERE id = ?1")
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("workspace not found".into()))?;
    Ok(PathBuf::from(folder))
}

fn resolve(root: &Path, path: &str) -> Result<PathBuf, Error> {
    let candidate = {
        let given = Path::new(path);
        if given.is_absolute() {
            given.to_path_buf()
        } else {
            root.join(given)
        }
    };
    harbor_paths::assert_within(root, &candidate).map_err(|error| Error::Message(error.to_string()))
}

pub async fn read(pool: &SqlitePool, workspace_id: &str, path: &str) -> Result<String, Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let resolved = resolve(&root, path)?;
    Ok(fs::read_to_string(resolved)?)
}

pub async fn write(
    pool: &SqlitePool,
    workspace_id: &str,
    path: &str,
    contents: &str,
) -> Result<(), Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let resolved = resolve(&root, path)?;
    fs::write(resolved, contents)?;
    Ok(())
}

pub async fn list(
    pool: &SqlitePool,
    workspace_id: &str,
    path: &str,
) -> Result<Vec<FsEntry>, Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let resolved = if path.is_empty() || path == "." {
        root.canonicalize()
            .map_err(|_| Error::Message("workspace missing".into()))?
    } else {
        resolve(&root, path)?
    };
    let mut entries = Vec::new();
    for entry in fs::read_dir(resolved)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if fs::symlink_metadata(entry.path())
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            // Skip unresolvable / out-of-root symlinks.
            if harbor_paths::assert_within(&root, &entry.path()).is_err() {
                continue;
            }
        }
        entries.push(FsEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().display().to_string(),
            directory: meta.is_dir(),
        });
    }
    entries.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.cmp(&b.name)));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn read_write_list_stay_inside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ws");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.txt"), "hello").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let workspace = crate::workspaces::add(&pool, root.display().to_string())
            .await
            .unwrap();

        assert_eq!(read(&pool, &workspace.id, "a.txt").await.unwrap(), "hello");
        write(&pool, &workspace.id, "a.txt", "world").await.unwrap();
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "world");
        write(&pool, &workspace.id, "b.txt", "new").await.unwrap();
        assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "new");

        let listed = list(&pool, &workspace.id, "").await.unwrap();
        assert!(listed.iter().any(|entry| entry.name == "a.txt"));
        assert!(
            listed
                .iter()
                .any(|entry| entry.name == "sub" && entry.directory)
        );

        assert!(read(&pool, &workspace.id, "../secret").await.is_err());
        let outside = dir.path().join("outside.txt");
        fs::write(&outside, "no").unwrap();
        assert!(
            read(&pool, &workspace.id, outside.to_str().expect("utf-8 path"))
                .await
                .is_err()
        );
    }
}
