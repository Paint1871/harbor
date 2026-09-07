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

/// The local profile name Harbor suggests when the builder has not chosen one.
///
/// Derived from the OS account, so a fresh install shows a real name without an
/// account, a network call, or spawning a process. Harbor never sends this
/// anywhere; it only labels the footer and the settings page.
pub fn os_account_name() -> String {
    let raw = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default();
    let trimmed = raw.trim();
    // Windows domain accounts arrive as DOMAIN\user; keep the account part.
    let account = trimmed.rsplit(['\\', '/']).next().unwrap_or(trimmed).trim();
    if account.is_empty() || account.len() > 40 {
        return "Local".into();
    }
    let mut chars = account.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Local".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::os_account_name;

    #[test]
    fn account_name_is_capitalized_or_falls_back() {
        // The helper reads the ambient environment, so assert on its contract
        // rather than on one machine's account.
        let name = os_account_name();
        assert!(!name.is_empty());
        assert!(name.len() <= 40);
        assert!(!name.contains('\\'));
        assert!(!name.contains('/'));
        assert_eq!(name.trim(), name);
        let first = name.chars().next().unwrap();
        assert!(
            !first.is_lowercase(),
            "expected a capitalized name, got {name}"
        );
    }
}
