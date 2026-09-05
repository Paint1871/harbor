use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::Error;

pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<Value>, Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value_json FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((json,)) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub async fn set(pool: &SqlitePool, key: &str, value: &Value) -> Result<(), Error> {
    let json = serde_json::to_string(value)?;
    sqlx::query(
        "INSERT INTO settings(key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
    )
    .bind(key)
    .bind(json)
    .execute(pool)
    .await?;
    Ok(())
}
