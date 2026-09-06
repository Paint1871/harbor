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

async fn run_blocking<T, F>(work: F) -> Result<T, Error>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, Error> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| Error::Message(format!("filesystem worker failed: {error}")))?
}

pub async fn read(pool: &SqlitePool, workspace_id: &str, path: &str) -> Result<String, Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let path = path.to_string();
    run_blocking(move || {
        let resolved = resolve(&root, &path)?;
        Ok(fs::read_to_string(resolved)?)
    })
    .await
}

pub async fn write(
    pool: &SqlitePool,
    workspace_id: &str,
    path: &str,
    contents: &str,
) -> Result<(), Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let path = path.to_string();
    let contents = contents.to_string();
    run_blocking(move || {
        let resolved = resolve(&root, &path)?;
        fs::write(resolved, contents)?;
        Ok(())
    })
    .await
}

pub async fn list(
    pool: &SqlitePool,
    workspace_id: &str,
    path: &str,
) -> Result<Vec<FsEntry>, Error> {
    let root = workspace_folder(pool, workspace_id).await?;
    let path = path.to_string();
    run_blocking(move || {
        let root = root
            .canonicalize()
            .map_err(|_| Error::Message("workspace missing".into()))?;
        let resolved = if path.is_empty() || path == "." {
            root.clone()
        } else {
            resolve(&root, &path)?
        };
        let mut entries = Vec::new();
        for entry in fs::read_dir(resolved)? {
            let entry = entry?;
            let entry_path = entry.path();
            let link_meta = fs::symlink_metadata(&entry_path)?;
            if link_meta.file_type().is_symlink() {
                // Skip unresolvable / out-of-root symlinks.
                if harbor_paths::assert_within(&root, &entry_path).is_err() {
                    continue;
                }
            }
            // Read metadata after the symlink boundary check. Broken links
            // and links to a protected folder should not make the whole tree
            // disappear; they are simply omitted from the safe listing.
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            entries.push(FsEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry_path.display().to_string(),
                directory: meta.is_dir(),
            });
        }
        entries.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.cmp(&b.name)));
        Ok(entries)
    })
    .await
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
