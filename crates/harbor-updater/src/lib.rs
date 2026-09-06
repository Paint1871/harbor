//! GitHub Releases + baked minisign public key. Unsigned artifacts are refused.

pub const PLACEHOLDER_KEY: &str = include_str!("../../../apps/desktop/src-tauri/minisign.pub");
pub const RELEASES_REPO_ENV: &str = "HARBOR_UPDATE_REPO";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("unsigned or tampered update refused")]
    Unsigned,
    #[error("placeholder public key cannot authorize an update")]
    PlaceholderKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub available: bool,
    pub version: Option<String>,
}

impl UpdateCheck {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            version: None,
        }
    }
}

pub fn public_key() -> &'static str {
    PLACEHOLDER_KEY
}

pub fn configured_repo() -> Option<String> {
    std::env::var(RELEASES_REPO_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn latest_release_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/releases/latest")
}

pub fn verify_release(signature: &str, key: &str) -> Result<(), UpdateError> {
    if key.contains("placeholder") || !key.contains("untrusted comment: minisign public key") {
        return Err(UpdateError::PlaceholderKey);
    }
    if !signature.contains("untrusted comment: minisign signature") {
        return Err(UpdateError::Unsigned);
    }
    Err(UpdateError::Unsigned)
}

fn tag_is_newer(tag: &str, current: &str) -> bool {
    let tag = tag.trim().trim_start_matches('v');
    let current = current.trim().trim_start_matches('v');
    !tag.is_empty() && tag != current
}

pub fn parse_tag_name(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

pub fn minisig_asset_url(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let assets = value.get("assets")?.as_array()?;
    assets.iter().find_map(|asset| {
        let name = asset.get("name")?.as_str()?;
        if name.ends_with(".minisig") {
            asset
                .get("browser_download_url")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        } else {
            None
        }
    })
}

/// Fail-open: network errors, missing repo, unsigned feeds, and the placeholder
/// key never report an installable update.
pub fn check_repo(
    repo: Option<&str>,
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
) -> UpdateCheck {
    let Some(repo) = repo.map(str::trim).filter(|value| !value.is_empty()) else {
        return UpdateCheck::unavailable();
    };
    let body = match fetch(&latest_release_url(repo)) {
        Ok(body) => body,
        Err(_) => return UpdateCheck::unavailable(),
    };
    let Some(tag) = parse_tag_name(&body) else {
        return UpdateCheck::unavailable();
    };
    if !tag_is_newer(&tag, current_version) {
        return UpdateCheck::unavailable();
    }
    let Some(sig_url) = minisig_asset_url(&body) else {
        return UpdateCheck::unavailable();
    };
    let Ok(signature) = fetch(&sig_url) else {
        return UpdateCheck::unavailable();
    };
    match verify_release(&signature, public_key()) {
        Ok(()) => UpdateCheck {
            available: true,
            version: Some(tag.trim_start_matches('v').to_string()),
        },
        Err(_) => UpdateCheck::unavailable(),
    }
}

pub fn check_latest(
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
) -> UpdateCheck {
    check_repo(configured_repo().as_deref(), fetch, current_version)
}

pub fn refuse_install() -> UpdateError {
    match verify_release("", public_key()) {
        Err(error) => error,
        Ok(()) => UpdateError::Unsigned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_never_authorizes() {
        assert_eq!(
            verify_release("untrusted comment: minisign signature\nRWS", public_key()),
            Err(UpdateError::PlaceholderKey)
        );
        assert_eq!(refuse_install(), UpdateError::PlaceholderKey);
    }

    #[test]
    fn missing_repo_or_bad_feed_is_not_available() {
        assert_eq!(
            check_latest(|_| Err("network".into()), "0.1.0"),
            UpdateCheck::unavailable()
        );
        assert_eq!(
            check_repo(None, |_| Ok("{}".into()), "0.1.0"),
            UpdateCheck::unavailable()
        );
        assert_eq!(
            check_repo(Some("example/harbor"), |_| Err("network".into()), "0.1.0"),
            UpdateCheck::unavailable()
        );
        assert!(parse_tag_name("not json").is_none());
        assert_eq!(
            parse_tag_name(r#"{"tag_name":"v0.2.0"}"#).as_deref(),
            Some("v0.2.0")
        );
        assert!(minisig_asset_url(r#"{"assets":[]}"#).is_none());
        assert_eq!(
            minisig_asset_url(
                r#"{"assets":[{"name":"Harbor.dmg.minisig","browser_download_url":"https://example.invalid/a.minisig"}]}"#
            )
            .as_deref(),
            Some("https://example.invalid/a.minisig")
        );
    }

    #[test]
    fn unsigned_newer_release_is_not_installable() {
        let json = r#"{"tag_name":"v9.9.9","assets":[{"name":"Harbor.dmg.minisig","browser_download_url":"https://example.invalid/a.minisig"}]}"#;
        let check = check_repo(
            Some("example/harbor"),
            |url| {
                if url.contains("/releases/latest") {
                    Ok(json.into())
                } else {
                    Ok("not a minisign signature".into())
                }
            },
            "0.1.0",
        );
        assert_eq!(check, UpdateCheck::unavailable());
    }
}
