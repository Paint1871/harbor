//! Tokens live in the OS keyring. Engines never see them.

use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("no token stored")]
    Missing,
    #[error("token store unavailable")]
    Unavailable,
}

pub fn key_name(plugin_id: &str) -> String {
    format!("harbor.plugin.{plugin_id}")
}

pub fn looks_like_github_user_token(token: &str) -> bool {
    token.starts_with("ghu_") || token.starts_with("gho_")
}

pub fn store_os(plugin_id: &str, token: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new("app.harbor.desktop", &key_name(plugin_id))
        .map_err(|_| KeyringError::Unavailable)?;
    entry
        .set_password(token)
        .map_err(|_| KeyringError::Unavailable)
}

pub fn load_os(plugin_id: &str) -> Result<String, KeyringError> {
    let entry = keyring::Entry::new("app.harbor.desktop", &key_name(plugin_id))
        .map_err(|_| KeyringError::Unavailable)?;
    entry.get_password().map_err(|_| KeyringError::Missing)
}

pub fn delete_os(plugin_id: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new("app.harbor.desktop", &key_name(plugin_id))
        .map_err(|_| KeyringError::Unavailable)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(KeyringError::Unavailable),
    }
}

/// 0600 file under the app-support keyring directory. Used when the OS
/// keychain is unavailable (CI, locked session).
pub fn store_file(dir: &Path, plugin_id: &str, token: &str) -> Result<(), KeyringError> {
    fs::create_dir_all(dir).map_err(|_| KeyringError::Unavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    let path = dir.join(plugin_id);
    fs::write(&path, token).map_err(|_| KeyringError::Unavailable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn load_file(dir: &Path, plugin_id: &str) -> Result<String, KeyringError> {
    fs::read_to_string(dir.join(plugin_id)).map_err(|_| KeyringError::Missing)
}

pub fn delete_file(dir: &Path, plugin_id: &str) -> Result<(), KeyringError> {
    match fs::remove_file(dir.join(plugin_id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(KeyringError::Unavailable),
    }
}

pub fn store(dir: &Path, plugin_id: &str, token: &str) -> Result<(), KeyringError> {
    match store_os(plugin_id, token) {
        Ok(()) => Ok(()),
        Err(_) => store_file(dir, plugin_id, token),
    }
}

pub fn load(dir: &Path, plugin_id: &str) -> Result<String, KeyringError> {
    match load_os(plugin_id) {
        Ok(token) => Ok(token),
        Err(_) => load_file(dir, plugin_id),
    }
}

pub fn delete(dir: &Path, plugin_id: &str) -> Result<(), KeyringError> {
    let _ = delete_os(plugin_id);
    delete_file(dir, plugin_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_store_roundtrip_and_github_prefix() {
        assert!(looks_like_github_user_token("ghu_abc"));
        let dir = std::env::temp_dir().join(format!("harbor-keyring-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        store_file(&dir, "github", "ghu_abc").unwrap();
        assert_eq!(load_file(&dir, "github").unwrap(), "ghu_abc");
        delete_file(&dir, "github").unwrap();
        assert!(load_file(&dir, "github").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
