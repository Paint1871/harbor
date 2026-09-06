pub mod approvals;
pub mod github;
pub mod keyring;
pub mod proxy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRow {
    pub id: &'static str,
    pub display_name: &'static str,
    pub status: &'static str,
}

pub fn listed_plugins() -> Vec<PluginRow> {
    [
        ("x", "X"),
        ("apollo", "Apollo"),
        ("vidiq", "vidIQ"),
        ("higgsfield", "Higgsfield"),
        ("fal", "fal"),
        ("youtube", "YouTube"),
        ("github", "GitHub"),
        ("linear", "Linear"),
        ("stripe", "Stripe"),
        ("cloudflare", "Cloudflare"),
        ("gmail", "Gmail"),
        ("supabase", "Supabase"),
        ("vercel", "Vercel"),
        ("shopify", "Shopify"),
        ("slack", "Slack"),
        ("notion", "Notion"),
    ]
    .into_iter()
    .map(|(id, display_name)| PluginRow {
        id,
        display_name,
        status: "available",
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_the_local_connection_providers() {
        let rows = listed_plugins();
        assert_eq!(rows.len(), 16);
        assert_eq!(rows[0].id, "x");
        assert!(rows.iter().any(|row| row.id == "linear"));
        assert!(rows.iter().any(|row| row.id == "slack"));
        assert!(rows.iter().any(|row| row.id == "notion"));
        assert!(rows.iter().any(|row| row.id == "vercel"));
        assert!(rows.iter().any(|row| row.id == "shopify"));
    }
}
