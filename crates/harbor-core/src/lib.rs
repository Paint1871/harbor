//! Local application core for Harbor.

pub mod agents;
pub mod commands;
pub mod db;
pub mod engines;
pub mod error;
pub mod restore;
pub mod settings;
pub mod threads;
pub mod types;
pub mod workspaces;

pub use sqlx::SqlitePool;

/// The version compiled into this crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::Row;

    #[tokio::test]
    async fn settings_roundtrip_and_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("harbor.sqlite");
        let pool = db::open(&path).await.unwrap();

        assert!(
            commands::settings_get(&pool, "onboarded_local")
                .await
                .unwrap()
                .is_null()
        );
        commands::settings_set(&pool, "onboarded_local", json!(true))
            .await
            .unwrap();
        commands::settings_set(&pool, "local_profile_name", json!("Builder"))
            .await
            .unwrap();
        assert_eq!(
            commands::settings_get(&pool, "onboarded_local")
                .await
                .unwrap(),
            json!(true)
        );
        assert_eq!(
            commands::settings_get(&pool, "local_profile_name")
                .await
                .unwrap(),
            json!("Builder")
        );

        let tables: Vec<String> =
            sqlx::query("SELECT name FROM sqlite_master WHERE type IN ('table', 'view')")
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();
        assert!(tables.contains(&"acp_permissions".into()));
        assert!(tables.contains(&"worktree_lanes".into()));
        assert!(tables.contains(&"messages_fts".into()));
        assert!(tables.contains(&"workspace_tabs".into()));
        assert!(!tables.iter().any(|name| name.contains("thread_messages")));

        let columns: Vec<String> =
            sqlx::query("SELECT name FROM pragma_table_info('workspace_tabs')")
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();
        assert!(columns.contains(&"layout_json".into()));
        let chat_columns: Vec<String> =
            sqlx::query("SELECT name FROM pragma_table_info('agent_chats')")
                .fetch_all(&pool)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();
        assert!(chat_columns.contains(&"config_json".into()));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[tokio::test]
    async fn host_commands_do_not_require_auth() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("harbor.sqlite")).await.unwrap();
        let engines = commands::engines_detect(&pool).await.unwrap();
        assert!(!engines.is_empty());
        let _agents = commands::agent_list(&pool).await.unwrap();
        let message = commands::pty_spawn(&pool, "pane".into(), "/tmp".into(), None)
            .await
            .expect_err("unimplemented host command")
            .to_string();
        assert!(
            message.contains("unimplemented"),
            "command failed for a reason other than unimplemented: {message}"
        );
        assert!(
            !message.to_ascii_lowercase().contains("token")
                && !message.to_ascii_lowercase().contains("entitlement")
                && !message.to_ascii_lowercase().contains("auth"),
            "{message}"
        );
    }
}
