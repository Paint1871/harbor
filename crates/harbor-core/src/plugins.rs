use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    agents,
    error::Error,
    types::{PluginApproval, PluginGrant, PluginRow},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub auth_kind: &'static str,
}

const PLUGIN_CATALOG: &[PluginDefinition] = &[
    PluginDefinition {
        id: "x",
        display_name: "X",
        description: "Post, reply, and read your timeline.",
        category: "Social",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "apollo",
        display_name: "Apollo",
        description: "Find leads and enrich contacts mid-task.",
        category: "Sales",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "vidiq",
        display_name: "vidIQ",
        description: "Research keywords and read your channel stats.",
        category: "Video",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "higgsfield",
        display_name: "Higgsfield",
        description: "Generate images and videos with your Higgsfield credits.",
        category: "Creative",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "fal",
        display_name: "fal",
        description: "Generate images and videos with your fal.ai key.",
        category: "Creative",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "youtube",
        display_name: "YouTube",
        description: "Read and manage the connected channel.",
        category: "Video",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "github",
        display_name: "GitHub",
        description: "Read repos, issues, and pull requests with a GitHub PAT.",
        category: "Development",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "linear",
        display_name: "Linear",
        description: "Find, create, and update issues, projects, and comments.",
        category: "Planning",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "stripe",
        display_name: "Stripe",
        description: "Read customers, invoices, and the catalog. Charges and refunds stay blocked.",
        category: "Commerce",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "cloudflare",
        display_name: "Cloudflare",
        description: "Manage Workers, DNS, R2, and D1 on your account.",
        category: "Development",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "gmail",
        display_name: "Gmail",
        description: "Read and send mail on the connected Google account.",
        category: "Communication",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "supabase",
        display_name: "Supabase",
        description: "Inspect and change the builder's Supabase projects.",
        category: "Development",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "vercel",
        display_name: "Vercel",
        description: "Manage projects, deployments, and domains.",
        category: "Development",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "shopify",
        display_name: "Shopify",
        description: "Manage products, orders, and inventory.",
        category: "Commerce",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "slack",
        display_name: "Slack",
        description: "Read channels and post with builder approval.",
        category: "Communication",
        auth_kind: "token",
    },
    PluginDefinition {
        id: "notion",
        display_name: "Notion",
        description: "Search, read, and update pages in your Notion workspace.",
        category: "Knowledge",
        auth_kind: "token",
    },
];

pub fn plugin_catalog() -> &'static [PluginDefinition] {
    PLUGIN_CATALOG
}

pub fn definition(id: &str) -> Option<&'static PluginDefinition> {
    PLUGIN_CATALOG.iter().find(|plugin| plugin.id == id)
}

fn display_name(id: &str) -> String {
    definition(id)
        .map(|plugin| plugin.display_name.to_string())
        .unwrap_or_else(|| id.to_string())
}

pub fn supports_manual_token(id: &str) -> bool {
    definition(id).is_some_and(|plugin| plugin.auth_kind == "token")
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<PluginRow>, Error> {
    let stored = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT id, status, display_name, account_label FROM plugins",
    )
    .fetch_all(pool)
    .await?;
    let stored_by_id: HashMap<_, _> = stored
        .into_iter()
        .map(|(id, status, display_name, account_label)| {
            (id, (status, display_name, account_label))
        })
        .collect();

    Ok(PLUGIN_CATALOG
        .iter()
        .map(|plugin| {
            let stored = stored_by_id.get(plugin.id);
            PluginRow {
                id: plugin.id.to_string(),
                display_name: stored
                    .map(|(_, display_name, _)| display_name.clone())
                    .unwrap_or_else(|| plugin.display_name.to_string()),
                status: stored
                    .map(|(status, _, _)| status.clone())
                    .unwrap_or_else(|| "available".into()),
                account_label: stored.and_then(|(_, _, account_label)| account_label.clone()),
                description: plugin.description.to_string(),
                category: plugin.category.to_string(),
                auth_kind: plugin.auth_kind.to_string(),
            }
        })
        .collect())
}

/// Record a connect request. Does not run OAuth or store tokens.
pub async fn connect(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    if id.trim().is_empty() {
        return Err(Error::Message("plugin id required".into()));
    }
    if definition(id).is_none() {
        return Err(Error::Message(format!("unsupported plugin: {id}")));
    }
    sqlx::query(
        "INSERT INTO plugins (id, display_name, status)
         VALUES (?1, ?2, 'connecting')
         ON CONFLICT(id) DO UPDATE SET status = 'connecting'",
    )
    .bind(id)
    .bind(display_name(id))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_connected(
    pool: &SqlitePool,
    id: &str,
    display_name: &str,
    account_label: Option<&str>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO plugins (id, display_name, status, account_label, connected_at)
         VALUES (?1, ?2, 'connected', ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
            status = 'connected',
            display_name = excluded.display_name,
            account_label = excluded.account_label,
            connected_at = excluded.connected_at",
    )
    .bind(id)
    .bind(display_name)
    .bind(account_label)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_disconnected(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    sqlx::query(
        "UPDATE plugins SET status = 'available', account_label = NULL, connected_at = NULL WHERE id = ?1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn disconnect(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    mark_disconnected(pool, id).await
}

async fn ensure_plugin(pool: &SqlitePool, plugin_id: &str) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO plugins (id, display_name, status) VALUES (?1, ?2, 'available')
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(plugin_id)
    .bind(display_name(plugin_id))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_agent_grant(
    pool: &SqlitePool,
    agent_id: &str,
    plugin_id: &str,
    enabled: bool,
) -> Result<(), Error> {
    if plugin_id.trim().is_empty() {
        return Err(Error::Message("plugin id required".into()));
    }
    agents::require(pool, agent_id).await?;
    ensure_plugin(pool, plugin_id).await?;
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM plugin_grants WHERE plugin_id = ?1 AND agent_id = ?2")
            .bind(plugin_id)
            .bind(agent_id)
            .fetch_optional(pool)
            .await?;
    if let Some((id,)) = existing {
        sqlx::query("UPDATE plugin_grants SET enabled = ?1 WHERE id = ?2")
            .bind(i64::from(enabled))
            .bind(id)
            .execute(pool)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO plugin_grants (id, plugin_id, agent_id, enabled, scopes_json)
             VALUES (?1, ?2, ?3, ?4, '[]')",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(plugin_id)
        .bind(agent_id)
        .bind(i64::from(enabled))
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn resolve_approval(pool: &SqlitePool, id: &str, allow: bool) -> Result<(), Error> {
    let status = if allow { "allowed" } else { "denied" };
    let result = sqlx::query("UPDATE plugin_approvals SET status = ?1 WHERE id = ?2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("approval not found".into()));
    }
    Ok(())
}

pub async fn list_approvals(pool: &SqlitePool) -> Result<Vec<PluginApproval>, Error> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
        "SELECT id, plugin_id, agent_id, action, status
         FROM plugin_approvals
         WHERE status = 'pending'
         ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, plugin_id, agent_id, action, status)| PluginApproval {
            id,
            plugin_id,
            agent_id,
            action,
            status,
        })
        .collect())
}

pub async fn list_grants(pool: &SqlitePool, agent_id: &str) -> Result<Vec<PluginGrant>, Error> {
    agents::require(pool, agent_id).await?;
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT plugin_id, enabled FROM plugin_grants WHERE agent_id = ?1",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(plugin_id, enabled)| PluginGrant {
            plugin_id,
            enabled: enabled != 0,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::types::CreateAgent;

    #[tokio::test]
    async fn connect_has_no_token_and_grants_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        connect(&pool, "github").await.unwrap();
        let rows = list(&pool).await.unwrap();
        assert_eq!(rows.len(), 16);
        assert_eq!(rows[6].status, "connecting");
        let linear = rows.iter().find(|row| row.id == "linear").unwrap();
        assert_eq!(linear.auth_kind, "token");
        assert_eq!(linear.category, "Planning");
        assert!(linear.account_label.is_none());
        let (account, connected_at): (Option<String>, Option<i64>) =
            sqlx::query_as("SELECT account_label, connected_at FROM plugins WHERE id = 'github'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(account.is_none());
        assert!(connected_at.is_none());

        disconnect(&pool, "github").await.unwrap();
        assert_eq!(list(&pool).await.unwrap()[0].status, "available");

        let agent = crate::agents::create(
            &pool,
            CreateAgent {
                name: "Plug".into(),
                brief: String::new(),
                engine_id: "opencode".into(),
                face_index: 0,
            },
        )
        .await
        .unwrap();
        set_agent_grant(&pool, &agent.id, "github", true)
            .await
            .unwrap();
        set_agent_grant(&pool, &agent.id, "github", false)
            .await
            .unwrap();
        let (count, enabled): (i64, i64) =
            sqlx::query_as("SELECT COUNT(*), MAX(enabled) FROM plugin_grants WHERE agent_id = ?1")
                .bind(&agent.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
        assert_eq!(enabled, 0);

        let approval_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO plugin_approvals (id, plugin_id, agent_id, action, payload_json, status, created_at)
             VALUES (?1, 'github', ?2, 'write', '{}', 'pending', ?3)",
        )
        .bind(&approval_id)
        .bind(&agent.id)
        .bind(now())
        .execute(&pool)
        .await
        .unwrap();
        let pending = list_approvals(&pool).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, approval_id);
        resolve_approval(&pool, &approval_id, true).await.unwrap();
        let (status,): (String,) =
            sqlx::query_as("SELECT status FROM plugin_approvals WHERE id = ?1")
                .bind(&approval_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "allowed");
        assert!(list_approvals(&pool).await.unwrap().is_empty());

        let grants = list_grants(&pool, &agent.id).await.unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].plugin_id, "github");
        assert!(!grants[0].enabled);
    }
}
