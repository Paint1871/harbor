use sqlx::SqlitePool;

use crate::error::Error;

pub async fn permission_resolve(
    pool: &SqlitePool,
    id: &str,
    option_id: Option<&str>,
    cancelled: bool,
) -> Result<(), Error> {
    let (status, selected) = if cancelled || option_id.is_none() {
        ("cancelled", None)
    } else {
        ("selected", option_id)
    };
    let result = sqlx::query(
        "UPDATE acp_permissions SET status = ?1, selected_option_id = ?2 WHERE id = ?3",
    )
    .bind(status)
    .bind(selected)
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("permission not found".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use uuid::Uuid;

    #[tokio::test]
    async fn resolve_selects_or_cancels() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO acp_permissions
                (id, session_ref, session_kind, options_json, status, created_at)
             VALUES (?1, 'sess', 'thread', '[]', 'pending', 1)",
        )
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
        permission_resolve(&pool, &id, Some("allow_once"), false)
            .await
            .unwrap();
        let (status, selected): (String, Option<String>) =
            sqlx::query_as("SELECT status, selected_option_id FROM acp_permissions WHERE id = ?1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "selected");
        assert_eq!(selected.as_deref(), Some("allow_once"));

        permission_resolve(&pool, &id, None, true).await.unwrap();
        let (status,): (String,) =
            sqlx::query_as("SELECT status FROM acp_permissions WHERE id = ?1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "cancelled");
        assert!(
            permission_resolve(&pool, "missing", None, true)
                .await
                .is_err()
        );
    }
}
