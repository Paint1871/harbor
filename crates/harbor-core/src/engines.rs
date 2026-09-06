use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::types::{DetectedEngine, EngineSpec};

const CATALOG_JSON: &str = include_str!("../../../packages/engine-catalog/src/catalog.json");

pub fn catalog() -> Vec<EngineSpec> {
    serde_json::from_str(CATALOG_JSON).expect("engine catalog JSON")
}

/// Recheck without ACP handshake: binary existence on PATH only.
pub fn detect_engines(search_path: &str) -> Vec<DetectedEngine> {
    let cwd = env::current_dir().ok();
    catalog()
        .into_iter()
        .map(|spec| detect_one(&spec, search_path, cwd.as_deref()))
        .collect()
}

pub fn recheck() -> Vec<DetectedEngine> {
    detect_engines(&runtime_path())
}

/// GUI apps on macOS do not always inherit the interactive shell's PATH.
/// Include the common user-managed CLI locations so a CLI installed through
/// Homebrew, Volta, Bun, npm, or nvm is discoverable without asking the user
/// to edit Harbor's environment manually.
fn runtime_path() -> String {
    let mut entries = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut candidates = Vec::new();
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.extend([
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
            home.join(".volta/bin"),
            home.join(".bun/bin"),
            home.join(".asdf/shims"),
            home.join(".nvm/current/bin"),
        ]);
        if let Ok(versions) = fs::read_dir(home.join(".nvm/versions/node")) {
            for version in versions.flatten() {
                candidates.push(version.path().join("bin"));
            }
        }
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/local/bin"),
    ]);
    for candidate in candidates {
        if candidate.is_dir() && !entries.iter().any(|entry| entry == &candidate) {
            entries.push(candidate);
        }
    }
    env::join_paths(entries)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn detect_one(spec: &EngineSpec, search_path: &str, cwd: Option<&Path>) -> DetectedEngine {
    let resolved: Vec<PathBuf> = spec
        .binaries
        .iter()
        .filter_map(|name| resolve_on_path(name, search_path, cwd))
        .collect();
    let found = resolved.first().cloned();
    let adapter = if spec.chat_mode == "adapter" && spec.binaries.len() > 1 {
        spec.binaries
            .iter()
            .skip(1)
            .find_map(|name| resolve_on_path(name, search_path, cwd))
    } else {
        None
    };

    let status: String = if found.is_none() {
        "cli-missing".into()
    } else if spec.chat_mode == "adapter" && spec.binaries.len() > 1 && adapter.is_none() {
        "adapter-missing".into()
    } else {
        "ready".into()
    };

    let supports_chat = status == "ready" && spec.acp_args.is_some();

    DetectedEngine {
        id: spec.id.clone(),
        display_name: spec.display_name.clone(),
        path: found
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        status,
        supports_chat,
        supports_terminal: spec.supports_terminal && found.is_some(),
    }
}

/// Absolute PATH entries only. Never cwd, never relative entries (Windows T10).
pub fn resolve_on_path(name: &str, search_path: &str, cwd: Option<&Path>) -> Option<PathBuf> {
    if name.contains(['/', '\\']) || name == "." || name == ".." || name.is_empty() {
        return None;
    }
    let cwd = cwd.and_then(|path| path.canonicalize().ok());
    env::split_paths(search_path).find_map(|directory| {
        if !directory.is_absolute() {
            return None;
        }
        let directory = directory.canonicalize().ok()?;
        if cwd.as_ref().is_some_and(|cwd| directory == *cwd) {
            return None;
        }
        let candidate = directory.join(name);
        file_on_path(&candidate)
    })
}

fn file_on_path(path: &Path) -> Option<PathBuf> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    path.canonicalize().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_the_single_table() {
        let specs = catalog();
        assert_eq!(specs.len(), 14);
        assert!(specs.iter().any(|spec| spec.id == "opencode"));
        assert!(specs.iter().all(|spec| spec.id != "windsurf"));
        assert_eq!(
            specs
                .iter()
                .find(|spec| spec.id == "opencode")
                .unwrap()
                .acp_args,
            Some(vec!["acp".into()])
        );
    }

    #[test]
    fn missing_binary_is_cli_missing() {
        let found = detect_engines("");
        assert!(found.iter().all(|engine| engine.status == "cli-missing"));
        assert!(found.iter().all(|engine| !engine.supports_chat));
    }

    #[test]
    fn installed_acp_engine_is_available_for_chat() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("opencode");
        fs::write(&bin, "fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let engines = detect_engines(&root.path().display().to_string());
        let engine = engines
            .iter()
            .find(|engine| engine.id == "opencode")
            .unwrap();
        assert_eq!(engine.status, "ready");
        assert!(engine.supports_chat);
        assert!(engine.supports_terminal);
        assert!(
            engines
                .iter()
                .filter(|engine| engine.status != "ready")
                .all(|engine| !engine.supports_chat)
        );
    }

    #[test]
    fn terminal_cli_remains_available_when_chat_adapter_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("claude");
        fs::write(&bin, "fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let engine = detect_engines(&root.path().display().to_string())
            .into_iter()
            .find(|engine| engine.id == "claude-code")
            .unwrap();
        assert_eq!(engine.status, "adapter-missing");
        assert!(engine.supports_terminal);
        assert!(!engine.supports_chat);
    }

    #[test]
    fn path_search_skips_relative_and_cwd() {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("cwd");
        let trusted = root.path().join("trusted");
        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&trusted).unwrap();
        let plant = |dir: &Path| {
            let bin = dir.join("opencode");
            fs::write(&bin, "fixture").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
            }
        };
        plant(&cwd);
        plant(&trusted);
        let paths = env::join_paths([&cwd, Path::new("."), &trusted]).unwrap();
        let found = resolve_on_path("opencode", &paths.to_string_lossy(), Some(&cwd)).unwrap();
        assert_eq!(found, trusted.join("opencode").canonicalize().unwrap());
    }
}
