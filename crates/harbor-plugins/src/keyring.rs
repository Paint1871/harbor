//! Tokens live in the OS keyring. Engines never see them.

#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("no token stored")]
    Missing,
}

pub fn key_name(plugin_id: &str) -> String {
    format!("harbor.plugin.{plugin_id}")
}

pub fn looks_like_github_user_token(token: &str) -> bool {
    token.starts_with("ghu_")
}
