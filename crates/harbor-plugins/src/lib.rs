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
    vec![PluginRow {
        id: "github",
        display_name: "GitHub",
        status: "available",
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_is_the_only_listed_provider() {
        let rows = listed_plugins();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "github");
        assert!(rows.iter().all(|row| row.id != "discord"));
    }
}
