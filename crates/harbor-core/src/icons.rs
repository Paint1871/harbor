//! Optional engine logos loaded from disk at runtime.
//!
//! Harbor ships original marks for every engine. Vendor logos are trademarks
//! that Harbor does not redistribute, so a builder who wants the real artwork
//! places it in the icon directory themselves and Harbor picks it up. Nothing
//! here is bundled into the binary or committed to the repository.

use std::path::Path;

use crate::error::Error;

/// A single logo is chrome, not content: refuse anything large enough to
/// suggest the directory is being used for something else.
const MAX_ICON_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineIcon {
    pub engine_id: String,
    /// A `data:` URL the renderer can put straight into an `<img>`.
    pub data_url: String,
}

fn mime_for(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Reads `<dir>/<engine-id>.<svg|png|webp>` for engines the catalog knows.
///
/// Only catalog ids are accepted, so an unexpected file in the directory cannot
/// introduce an engine that Harbor does not otherwise support.
pub fn engine_icons(dir: &Path) -> Result<Vec<EngineIcon>, Error> {
    let known: Vec<String> = crate::engines::catalog()
        .into_iter()
        .map(|spec| spec.id)
        .collect();
    let mut icons = Vec::new();
    for id in known {
        for extension in ["svg", "png", "webp"] {
            let candidate = dir.join(format!("{id}.{extension}"));
            let Ok(metadata) = std::fs::metadata(&candidate) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ICON_BYTES {
                continue;
            }
            let Some(mime) = mime_for(extension) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(&candidate) else {
                continue;
            };
            icons.push(EngineIcon {
                engine_id: id.clone(),
                data_url: format!("data:{mime};base64,{}", crate::b64::encode(&bytes)),
            });
            break;
        }
    }
    Ok(icons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_catalog_engines_only_and_prefers_svg() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("codex.svg"), b"<svg/>").unwrap();
        std::fs::write(root.join("codex.png"), b"png-bytes").unwrap();
        std::fs::write(root.join("cursor.png"), b"png-bytes").unwrap();
        // Not a catalog id, and an extension Harbor does not accept.
        std::fs::write(root.join("not-an-engine.svg"), b"<svg/>").unwrap();
        std::fs::write(root.join("codex.exe"), b"nope").unwrap();

        let icons = engine_icons(root).unwrap();
        let ids: Vec<&str> = icons.iter().map(|i| i.engine_id.as_str()).collect();
        assert_eq!(ids, vec!["codex", "cursor"]);

        let codex = &icons[0];
        assert!(
            codex.data_url.starts_with("data:image/svg+xml;base64,"),
            "svg wins over png, got {}",
            codex.data_url
        );
    }

    #[test]
    fn skips_empty_and_oversized_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("codex.svg"), b"").unwrap();
        std::fs::write(
            root.join("cursor.png"),
            vec![0u8; (MAX_ICON_BYTES + 1) as usize],
        )
        .unwrap();
        assert!(engine_icons(root).unwrap().is_empty());
    }

    #[test]
    fn a_missing_directory_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("engine-icons");
        assert!(engine_icons(&missing).unwrap().is_empty());
    }
}
