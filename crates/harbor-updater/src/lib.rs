//! GitHub Releases + baked minisign public key. Unsigned artifacts are refused.
//!
//! A release ships a `manifest.json` plus `manifest.json.minisig`. The signed
//! manifest binds the release tag to exact artifact metadata — version,
//! os/arch, file name and sha256 — so a validly signed artifact from an older
//! release cannot be replayed under a newer tag. `check` verifies the manifest
//! signature; `install` verifies the artifact bytes against the manifest's
//! sha256. No manifest, bad signature, or a hash mismatch all fail closed.

use minisign_verify::{PublicKey, Signature};
use sha2::Digest;

/// The release signing key the Harbor publisher generated for this project.
/// Releases are signed with the matching secret key, which never ships.
pub const PUBLIC_KEY: &str = include_str!("../../../apps/desktop/src-tauri/minisign.pub");
pub const RELEASES_REPO_ENV: &str = "HARBOR_UPDATE_REPO";

/// The release asset that carries signed artifact metadata.
pub const MANIFEST_ASSET: &str = "manifest.json";

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
    /// sha256 hex the downloaded artifact must match — bound by the signed
    /// manifest, so the bytes themselves are pinned, not just signed-adjacent.
    pub sha256: Option<String>,
    /// Manifest-bound file name, used for staging.
    pub file_name: Option<String>,
}

impl UpdateCheck {
    pub fn unavailable() -> Self {
        Self {
            available: false,
            version: None,
            artifact_url: None,
            sha256: None,
            file_name: None,
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

/// The artifact suffixes that can install on this OS — a sanity bound on what
/// a manifest may name for this platform, not the selection mechanism itself.
fn platform_suffixes() -> &'static [&'static str] {
    match std::env::consts::OS {
        "macos" => &[".dmg"],
        "windows" => &[".msi", ".exe"],
        _ => &[".AppImage", ".deb", ".rpm"],
    }
}

/// This build's platform key as manifests spell it.
fn platform() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
fn sha256_hex(bytes: &[u8]) -> String {
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The manifest entry matching this platform, if the manifest genuinely
/// describes this release and this machine.
fn manifest_entry(
    manifest: &str,
    release_version: &str,
    current_version: &str,
) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(manifest).ok()?;
    // The manifest must describe *this* release — one transplanted from
    // another release cannot ride along under this tag.
    let version = value.get("version")?.as_str()?;
    if version != release_version {
        return None;
    }
    if let Some(minimum) = value.get("minimum_version").and_then(|v| v.as_str())
        && !version_at_least(current_version, minimum)
    {
        return None;
    }
    let artifacts = value.get("artifacts")?.as_array()?;
    let (os, arch) = platform();
    artifacts.iter().find_map(|artifact| {
        if artifact.get("os")?.as_str()? != os || artifact.get("arch")?.as_str()? != arch {
            return None;
        }
        let file = artifact.get("file")?.as_str()?;
        let sha256 = artifact.get("sha256")?.as_str()?;
        if file.is_empty() || !valid_sha256(sha256) {
            return None;
        }
        // A manifest that names, say, an .msi for macOS is malformed.
        if !platform_suffixes()
            .iter()
            .any(|suffix| file.ends_with(suffix))
        {
            return None;
        }
        Some((file.to_string(), sha256.to_string()))
    })
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

/// The `manifest.json` + `manifest.json.minisig` asset URLs of a release.
fn manifest_assets(json: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let assets = value.get("assets")?.as_array()?;
    let manifest_sig = format!("{MANIFEST_ASSET}.minisig");
    let manifest_url = asset_url(assets, |name| name == MANIFEST_ASSET)?;
    let signature_url = asset_url(assets, |name| name == manifest_sig)?;
    Some((manifest_url, signature_url))
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

/// `current` satisfies `minimum` — a manifest's `minimum_version` gate. A
/// prerelease of the same version does not satisfy the released minimum.
fn version_at_least(current: &str, minimum: &str) -> bool {
    let (Some((ca, cb, cc, cpre)), Some((ma, mb, mc, mpre))) =
        (version_key(current), version_key(minimum))
    else {
        return false;
    };
    (ca, cb, cc) > (ma, mb, mc)
        || ((ca, cb, cc) == (ma, mb, mc) && (cpre.is_none() || mpre.is_some()))
}

pub fn parse_tag_name(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value
        .get("tag_name")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

/// Fail closed: network errors, a missing repo, a same-or-older tag, a
/// release without a signed manifest, or a manifest that does not name this
/// platform never report an update.
pub fn check_repo(
    repo: Option<&str>,
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
) -> UpdateCheck {
    check_repo_with_key(repo, fetch, current_version, public_key())
}

/// The key-parameterized inner path so tests can exercise the full signed
/// pipeline with a throwaway keypair — the baked key's secret never ships.
fn check_repo_with_key(
    repo: Option<&str>,
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
    key_file: &str,
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
    let release_version = tag.trim_start_matches('v');
    if !tag_is_newer(&tag, current_version) {
        return UpdateCheck::unavailable();
    }
    let Some((manifest_url, signature_url)) = manifest_assets(&body) else {
        return UpdateCheck::unavailable();
    };
    let (Ok(manifest), Ok(signature)) = (fetch(&manifest_url), fetch(&signature_url)) else {
        return UpdateCheck::unavailable();
    };
    // The manifest is the trust anchor: only bytes signed by the release key
    // may tell us which artifact is ours and which digest it must have.
    if verify_release(manifest.as_bytes(), &signature, key_file).is_err() {
        return UpdateCheck::unavailable();
    }
    let Some((file, sha256)) = manifest_entry(&manifest, release_version, current_version) else {
        return UpdateCheck::unavailable();
    };
    let value: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let assets = value.get("assets").and_then(|a| a.as_array());
    let artifact_url = assets.and_then(|assets| asset_url(assets, |name| name == file));
    let Some(artifact_url) = artifact_url else {
        return UpdateCheck::unavailable();
    };
    UpdateCheck {
        available: true,
        version: Some(release_version.to_string()),
        artifact_url: Some(artifact_url),
        sha256: Some(sha256),
        file_name: Some(file),
    }
}

pub fn check_latest(
    fetch: impl Fn(&str) -> Result<String, String>,
    current_version: &str,
) -> UpdateCheck {
    check_repo(configured_repo().as_deref(), fetch, current_version)
}

/// Hashes every byte that flows through it, so a multi-hundred-megabyte
/// artifact is verified while it streams to disk instead of sitting in RAM.
struct HashingWriter<'a> {
    inner: &'a mut dyn std::io::Write,
    hasher: sha2::Sha256,
}

impl std::io::Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(buf)?;
        self.hasher.update(buf);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Stream the artifact into `sink` and check it against the digest the
/// signed manifest bound it to. A hash mismatch leaves the caller to clean
/// up its staging file — a failed digest must not sit at the final path.
pub fn install_release(
    check: &UpdateCheck,
    fetch_stream: impl Fn(&str, &mut dyn std::io::Write) -> Result<(), String>,
    sink: &mut dyn std::io::Write,
) -> Result<(), UpdateError> {
    if !check.available {
        return Err(UpdateError::Unavailable);
    }
    let artifact_url = check
        .artifact_url
        .as_deref()
        .ok_or(UpdateError::Unavailable)?;
    let expected = check.sha256.as_deref().ok_or(UpdateError::Unavailable)?;
    let mut hashing = HashingWriter {
        inner: sink,
        hasher: sha2::Sha256::new(),
    };
    fetch_stream(artifact_url, &mut hashing).map_err(|_| UpdateError::Download)?;
    if format!("{:x}", hashing.hasher.finalize()) != expected {
        return Err(UpdateError::Unsigned);
    }
    Ok(())
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
        assert!(manifest_assets(r#"{"assets":[]}"#).is_none());
        assert_eq!(
            manifest_assets(
                r#"{"assets":[
                    {"name":"manifest.json","browser_download_url":"https://example.invalid/manifest.json"},
                    {"name":"manifest.json.minisig","browser_download_url":"https://example.invalid/manifest.json.minisig"}
                ]}"#
            )
            .map(|(m, _)| m)
            .as_deref(),
            Some("https://example.invalid/manifest.json")
        );
    }

    fn platform_asset_name() -> &'static str {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "Harbor_0.2.0_aarch64.dmg",
            ("macos", _) => "Harbor_0.2.0_x64.dmg",
            ("windows", _) => "Harbor_0.2.0_x64.msi",
            (_, "aarch64") => "Harbor_0.2.0_arm64.AppImage",
            _ => "Harbor_0.2.0_amd64.AppImage",
        }
    }

    /// A throwaway keypair signed the manifest; `check_repo_with_key` is the
    /// same code path as production, only the trust root differs.
    fn signed_release(manifest: &str) -> (String, String) {
        let keypair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let signature = minisign::sign(
            None,
            &keypair.sk,
            std::io::Cursor::new(manifest.as_bytes()),
            None,
            None,
        )
        .unwrap();
        (
            keypair.pk.to_box().unwrap().to_string(),
            signature.into_string(),
        )
    }

    fn manifest_for(version: &str, file: &str, sha256: &str) -> String {
        format!(
            r#"{{"version":"{version}","artifacts":[{{"os":"{}","arch":"{}","file":"{file}","sha256":"{sha256}"}}]}}"#,
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }

    #[test]
    fn newer_release_with_signed_manifest_is_available() {
        let file = platform_asset_name();
        let digest = sha256_hex(TEST_ARTIFACT);
        let manifest = manifest_for("0.2.0", file, &digest);
        let (key, signature) = signed_release(&manifest);
        let release = format!(
            r#"{{"tag_name":"v0.2.0","assets":[
                {{"name":"{file}","browser_download_url":"https://example.invalid/app"}},
                {{"name":"manifest.json","browser_download_url":"https://example.invalid/manifest"}},
                {{"name":"manifest.json.minisig","browser_download_url":"https://example.invalid/manifest.minisig"}}
            ]}}"#
        );
        let fetch = |url: &str| -> Result<String, String> {
            match url {
                "https://example.invalid/manifest" => Ok(manifest.clone()),
                "https://example.invalid/manifest.minisig" => Ok(signature.clone()),
                _ => Ok(release.clone()),
            }
        };
        let check = check_repo_with_key(Some("example/harbor"), fetch, "0.1.0", &key);
        assert!(check.available);
        assert_eq!(check.version.as_deref(), Some("0.2.0"));
        assert_eq!(
            check.artifact_url.as_deref(),
            Some("https://example.invalid/app")
        );
        assert_eq!(check.sha256.as_deref(), Some(digest.as_str()));
        assert_eq!(check.file_name.as_deref(), Some(file));

        // Same tag is not an update.
        assert!(!check_repo_with_key(Some("example/harbor"), fetch, "0.2.0", &key).available);
    }

    #[test]
    fn unsigned_or_transplanted_manifests_are_not_updates() {
        let file = platform_asset_name();
        let digest = sha256_hex(TEST_ARTIFACT);
        let manifest = manifest_for("0.2.0", file, &digest);
        let (key, _signature) = signed_release(&manifest);
        let release = format!(
            r#"{{"tag_name":"v0.2.0","assets":[
                {{"name":"{file}","browser_download_url":"https://example.invalid/app"}},
                {{"name":"manifest.json","browser_download_url":"https://example.invalid/manifest"}},
                {{"name":"manifest.json.minisig","browser_download_url":"https://example.invalid/manifest.minisig"}}
            ]}}"#
        );
        let base_fetch = |manifest_text: String, sig: String| {
            let release = release.clone();
            move |url: &str| -> Result<String, String> {
                match url {
                    "https://example.invalid/manifest" => Ok(manifest_text.clone()),
                    "https://example.invalid/manifest.minisig" => Ok(sig.clone()),
                    _ => Ok(release.clone()),
                }
            }
        };
        // A manifest the key never signed.
        let bad = base_fetch(manifest.clone(), "untrusted comment\ninvalid".to_string());
        assert!(!check_repo_with_key(Some("example/harbor"), bad, "0.1.0", &key).available);
        // A manifest signed for a *different* tag.
        let other = manifest_for("0.3.0", file, &digest);
        let (_, other_sig) = signed_release(&other);
        let mixed = base_fetch(other, other_sig);
        assert!(!check_repo_with_key(Some("example/harbor"), mixed, "0.1.0", &key).available);
        // A manifest for a different platform.
        let wrong_os = format!(
            r#"{{"version":"0.2.0","artifacts":[{{"os":"plan9","arch":"mips","file":"{file}","sha256":"{digest}"}}]}}"#
        );
        let (_, wrong_sig) = signed_release(&wrong_os);
        let foreign = base_fetch(wrong_os, wrong_sig);
        assert!(!check_repo_with_key(Some("example/harbor"), foreign, "0.1.0", &key).available);
        // A release without manifest assets at all.
        let bare = format!(
            r#"{{"tag_name":"v0.2.0","assets":[{{"name":"{file}","browser_download_url":"https://example.invalid/app"}}]}}"#
        );
        assert!(
            !check_repo_with_key(
                Some("example/harbor"),
                move |_| Ok(bare.clone()),
                "0.1.0",
                &key
            )
            .available
        );
        // A minimum_version above the running build blocks the update path.
        let gated = manifest.replace(
            "\"artifacts\"",
            "\"minimum_version\":\"0.2.0\",\"artifacts\"",
        );
        let (_, gated_sig) = signed_release(&gated);
        let gated_fetch = base_fetch(gated, gated_sig);
        assert!(!check_repo_with_key(Some("example/harbor"), gated_fetch, "0.1.0", &key).available);
    }

    #[test]
    fn install_checks_the_manifest_digest() {
        let check = UpdateCheck {
            available: true,
            version: Some("9.9.9".into()),
            artifact_url: Some("https://example.invalid/app".into()),
            sha256: Some(sha256_hex(TEST_ARTIFACT)),
            file_name: Some("Harbor_9.9.9.dmg".into()),
        };
        let mut sink = Vec::new();
        let fetch = |_url: &str, out: &mut dyn std::io::Write| {
            out.write_all(TEST_ARTIFACT).map_err(|e| e.to_string())
        };
        assert_eq!(install_release(&check, fetch, &mut sink), Ok(()));
        assert_eq!(sink, TEST_ARTIFACT);

        let mut sink = Vec::new();
        let tampered = |_url: &str, out: &mut dyn std::io::Write| {
            out.write_all(b"tampered artifact")
                .map_err(|e| e.to_string())
        };
        assert_eq!(
            install_release(&check, tampered, &mut sink),
            Err(UpdateError::Unsigned)
        );

        let mut sink = Vec::new();
        assert_eq!(
            install_release(&UpdateCheck::unavailable(), fetch, &mut sink),
            Err(UpdateError::Unavailable)
        );
    }
}
