//! GitHub Releases + baked minisign public key. Unsigned artifacts are refused.
//!
//! The check path only reads release metadata; the install path downloads the
//! artifact and its `.minisig`, then verifies the minisign signature against
//! the public key baked into the binary. No signature, wrong signature, or an
//! unconfigured key all fail closed.

use minisign_verify::{PublicKey, Signature};

/// The release signing key the Harbor publisher generated for this project.
/// Releases are signed with the matching secret key, which never ships.
pub const PUBLIC_KEY: &str = include_str!("../../../apps/desktop/src-tauri/minisign.pub");
pub const RELEASES_REPO_ENV: &str = "HARBOR_UPDATE_REPO";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("unsigned or tampered update refused")]
    Unsigned,
    #[error("no release signing key is configured in this build")]
    UnconfiguredKey,
    #[error("update download failed")]
    Download,
    #[error("no usable update is available")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub available: bool,
    pub version: Option<String>,
    /// Download URL of the artifact matching this platform.
    pub artifact_url: Option<String>,
    /// Download URL of its `.minisig` signature file.
    pub signature_url: Option<String>,
}

impl UpdateCheck {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            version: None,
            artifact_url: None,
            signature_url: None,
        }
    }
}

pub fn public_key() -> &'static str {
    PUBLIC_KEY
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

/// The artifact suffixes that can install on this OS.
fn platform_suffixes() -> &'static [&'static str] {
    match std::env::consts::OS {
        "macos" => &[".dmg"],
        "windows" => &[".msi", ".exe"],
        _ => &[".AppImage", ".deb", ".rpm"],
    }
}

fn asset_url(assets: &[serde_json::Value], pred: impl Fn(&str) -> bool) -> Option<String> {
    assets.iter().find_map(|asset| {
        let name = asset.get("name")?.as_str()?;
        if !pred(name) {
            return None;
        }
        // Downloads must stay on TLS; a metadata response pointing anywhere
        // else is treated as if the asset did not exist.
        asset
            .get("browser_download_url")
            .and_then(|value| value.as_str())
            .filter(|url| url.starts_with("https://"))
            .map(str::to_string)
    })
}

/// The artifact for this platform plus the `.minisig` that pairs with it.
fn release_assets(json: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let assets = value.get("assets")?.as_array()?;
    let artifact = asset_url(assets, |name| {
        platform_suffixes()
            .iter()
            .any(|suffix| name.ends_with(suffix))
    })?;
    let artifact_name = assets
        .iter()
        .filter_map(|asset| asset.get("name")?.as_str())
        .find(|name| platform_suffixes().iter().any(|s| name.ends_with(s)))?
        .to_string();
    let paired = format!("{artifact_name}.minisig");
    let signature = asset_url(assets, |name| name == paired)
        .or_else(|| asset_url(assets, |name| name.ends_with(".minisig")))?;
    Some((artifact, signature))
}

/// Verify `signature_file` (a `.minisig` payload) against `artifact` and
/// `key_file` (a minisign `.pub` payload). Real ed25519 verification, not a
/// string check: an unsigned or tampered artifact can never pass.
pub fn verify_release(
    artifact: &[u8],
    signature_file: &str,
    key_file: &str,
) -> Result<(), UpdateError> {
    if key_file.contains("UNCONFIGURED") || key_file.contains("placeholder") {
        return Err(UpdateError::UnconfiguredKey);
    }
    let public = PublicKey::decode(key_file).map_err(|_| UpdateError::UnconfiguredKey)?;
    let signature = Signature::decode(signature_file).map_err(|_| UpdateError::Unsigned)?;
    public
        .verify(artifact, &signature, false)
        .map_err(|_| UpdateError::Unsigned)
}

/// (major, minor, patch, prerelease) — enough semver for a release tag.
fn version_key(tag: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let tag = tag.trim().trim_start_matches('v');
    if tag.is_empty() {
        return None;
    }
    let (core, pre) = match tag.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_string())),
        None => (tag, None),
    };
    let parts: Vec<u64> = core
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [a, b, c] => Some((*a, *b, *c, pre)),
        [a, b] => Some((*a, *b, 0, pre)),
        [a] => Some((*a, 0, 0, pre)),
        _ => None,
    }
}

/// Strictly newer, semver-wise: `v0.9.9` no longer beats `0.10.0`, and a
/// prerelease of the same version counts as older than the release.
pub fn tag_is_newer(tag: &str, current: &str) -> bool {
    let (Some((ta, tb, tc, tpre)), Some((ca, cb, cc, cpre))) =
        (version_key(tag), version_key(current))
    else {
        return false;
    };
    (ta, tb, tc) > (ca, cb, cc)
        || ((ta, tb, tc) == (ca, cb, cc) && tpre.is_none() && cpre.is_some())
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
    asset_url(assets, |name| name.ends_with(".minisig"))
}

/// Fail closed: network errors, a missing repo, a same-or-older tag, or a
/// release without a matching artifact and `.minisig` never report an update.
/// Signature verification itself happens at install time, over real bytes.
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
    let Some((artifact_url, signature_url)) = release_assets(&body) else {
        return UpdateCheck::unavailable();
    };
    UpdateCheck {
        available: true,
        version: Some(tag.trim_start_matches('v').to_string()),
        artifact_url: Some(artifact_url),
        signature_url: Some(signature_url),
    }
}

pub fn check_latest(
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
) -> UpdateCheck {
    check_repo(configured_repo().as_deref(), fetch, current_version)
}

/// Download the artifact and its signature, then verify. Returns the verified
/// bytes so the caller decides where a signed update gets staged — anything
/// unsigned never touches the filesystem.
pub fn install_release(
    check: &UpdateCheck,
    fetch_bytes: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, UpdateError> {
    if !check.available {
        return Err(UpdateError::Unavailable);
    }
    let artifact_url = check
        .artifact_url
        .as_deref()
        .ok_or(UpdateError::Unavailable)?;
    let signature_url = check
        .signature_url
        .as_deref()
        .ok_or(UpdateError::Unavailable)?;
    let artifact = fetch_bytes(artifact_url).map_err(|_| UpdateError::Download)?;
    let signature = fetch_bytes(signature_url).map_err(|_| UpdateError::Download)?;
    let signature = String::from_utf8(signature).map_err(|_| UpdateError::Unsigned)?;
    verify_release(&artifact, &signature, public_key())?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signature of `b"harbor test artifact"` under the baked release key,
    /// produced once at key-generation time. Proves the embedded public key
    /// really verifies — the secret never ships.
    const TEST_ARTIFACT: &[u8] = b"harbor test artifact";
    const TEST_SIGNATURE: &str = include_str!("../test-vector.minisig");

    #[test]
    fn real_key_verifies_a_real_signature_and_refuses_tampering() {
        assert_eq!(
            verify_release(TEST_ARTIFACT, TEST_SIGNATURE, public_key()),
            Ok(())
        );
        // One flipped byte and the same signature no longer authorizes.
        assert_eq!(
            verify_release(b"harbor test artifaxt", TEST_SIGNATURE, public_key()),
            Err(UpdateError::Unsigned)
        );
        assert_eq!(
            verify_release(TEST_ARTIFACT, "not a signature", public_key()),
            Err(UpdateError::Unsigned)
        );
        assert_eq!(
            verify_release(TEST_ARTIFACT, TEST_SIGNATURE, "UNCONFIGURED placeholder"),
            Err(UpdateError::UnconfiguredKey)
        );
    }

    #[test]
    fn semver_newer_not_just_different() {
        assert!(tag_is_newer("v0.2.0", "0.1.0"));
        assert!(tag_is_newer("0.10.0", "v0.9.9"));
        assert!(tag_is_newer("v1.0.0", "1.0.0-beta.2"));
        assert!(!tag_is_newer("v0.1.0", "0.2.0"));
        assert!(!tag_is_newer("0.1.0", "0.1.0"));
        assert!(!tag_is_newer("v0.1.0-beta.1", "0.1.0"));
        assert!(!tag_is_newer("latest", "0.1.0"));
        assert!(!tag_is_newer("", "0.1.0"));
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

    fn platform_asset_name() -> &'static str {
        match std::env::consts::OS {
            "macos" => "Harbor_0.2.0_aarch64.dmg",
            "windows" => "Harbor_0.2.0_x64.msi",
            _ => "Harbor_0.2.0_amd64.AppImage",
        }
    }

    #[test]
    fn newer_release_with_paired_assets_is_available() {
        let artifact = platform_asset_name();
        let json = format!(
            r#"{{"tag_name":"v0.2.0","assets":[
                {{"name":"{artifact}","browser_download_url":"https://example.invalid/app"}},
                {{"name":"{artifact}.minisig","browser_download_url":"https://example.invalid/app.minisig"}}
            ]}}"#
        );
        let check = check_repo(Some("example/harbor"), |_| Ok(json.clone()), "0.1.0");
        assert!(check.available);
        assert_eq!(check.version.as_deref(), Some("0.2.0"));
        assert_eq!(
            check.artifact_url.as_deref(),
            Some("https://example.invalid/app")
        );
        assert_eq!(
            check.signature_url.as_deref(),
            Some("https://example.invalid/app.minisig")
        );

        // Same tag or a release without a paired .minisig is not an update.
        assert!(!check_repo(Some("example/harbor"), |_| Ok(json.clone()), "0.2.0").available);
        let unsigned = r#"{"tag_name":"v0.3.0","assets":[{"name":"Harbor_0.3.0.dmg","browser_download_url":"https://example.invalid/x"}]}"#;
        assert!(!check_repo(Some("example/harbor"), |_| Ok(unsigned.into()), "0.1.0").available);
    }

    #[test]
    fn install_verifies_before_returning_bytes() {
        let check = UpdateCheck {
            available: true,
            version: Some("9.9.9".into()),
            artifact_url: Some("https://example.invalid/app".into()),
            signature_url: Some("https://example.invalid/app.minisig".into()),
        };
        let installed = install_release(&check, |url| {
            if url.ends_with("minisig") {
                Ok(TEST_SIGNATURE.as_bytes().to_vec())
            } else {
                Ok(TEST_ARTIFACT.to_vec())
            }
        });
        assert_eq!(installed.as_deref(), Ok(TEST_ARTIFACT));

        let tampered = install_release(&check, |url| {
            if url.ends_with("minisig") {
                Ok(TEST_SIGNATURE.as_bytes().to_vec())
            } else {
                Ok(b"tampered artifact".to_vec())
            }
        });
        assert_eq!(tampered, Err(UpdateError::Unsigned));

        assert_eq!(
            install_release(&UpdateCheck::unavailable(), |_| Ok(Vec::new())),
            Err(UpdateError::Unavailable)
        );
    }
}
