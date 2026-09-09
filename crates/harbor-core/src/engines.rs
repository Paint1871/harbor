use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{OnceLock, mpsc},
    thread,
    time::Duration,
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
/// The user's login shell, if `$SHELL` names one the system actually lists.
///
/// `$SHELL` is attacker-controlled in the sense that anything in the
/// environment is, so Harbor only runs a program that appears in the system's
/// own shell registry. That keeps a poisoned `SHELL=/tmp/evil` from being
/// executed just because Harbor wanted a PATH.
fn login_shell(shell: Option<&Path>, registered: &[PathBuf]) -> Option<PathBuf> {
    let shell = shell?;
    if !shell.is_absolute() {
        return None;
    }
    registered
        .iter()
        .any(|entry| entry == shell)
        .then(|| shell.to_path_buf())
}

fn registered_shells() -> Vec<PathBuf> {
    fs::read_to_string("/etc/shells")
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Extracts the PATH the login shell reports, between two sentinels.
///
/// A login shell prints whatever the user's rc files print, so the value is
/// framed rather than read as "the output".
fn parse_probe(output: &str) -> Option<String> {
    let (_, rest) = output.split_once(PROBE_OPEN)?;
    let (value, _) = rest.split_once(PROBE_CLOSE)?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

const PROBE_OPEN: &str = "__harbor_path_begin__";
const PROBE_CLOSE: &str = "__harbor_path_end__";

/// Runs the login shell once to read the PATH the user actually has.
///
/// Engines are whatever is on the login-shell PATH. A GUI launch inherits
/// launchd's minimal PATH instead, so without this a CLI installed anywhere
/// unusual is invisible when Harbor is opened from the Dock.
fn probe_login_shell_path(shell: &Path) -> Option<String> {
    let fish = shell.file_name().and_then(|name| name.to_str()) == Some("fish");
    let script = if fish {
        format!("printf '{PROBE_OPEN}%s{PROBE_CLOSE}' (string join : $PATH)")
    } else {
        format!("printf '{PROBE_OPEN}%s{PROBE_CLOSE}' \"$PATH\"")
    };

    let mut command = Command::new(shell);
    command
        .arg("-l")
        .arg("-i")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        // Login rc files commonly branch on this; keep the probe non-interactive
        // in spirit and stop tools from paging output back at us.
        .env("HARBOR_PATH_PROBE", "1")
        .env("PAGER", "cat")
        .env("TERM", "dumb");

    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let mut reader = stdout;
        let _ = reader.read_to_string(&mut text);
        let _ = sender.send(text);
    });

    // A broken rc file can hang forever; a stuck probe must not stall startup.
    let text = match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(text) => text,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let _ = child.wait();
    parse_probe(&text)
}

fn login_shell_path() -> Option<&'static str> {
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let shell = env::var_os("SHELL").map(PathBuf::from);
            let shell = login_shell(shell.as_deref(), &registered_shells())?;
            probe_login_shell_path(&shell)
        })
        .as_deref()
}

/// Where Harbor installs the ACP adapters it manages itself. The host sets it
/// once at startup; without it, only adapters already on PATH are found.
static ADAPTER_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn set_adapter_root(dir: PathBuf) {
    let _ = ADAPTER_ROOT.set(dir);
}

pub fn adapter_root() -> Option<&'static PathBuf> {
    ADAPTER_ROOT.get()
}

fn adapter_bin_dir() -> Option<PathBuf> {
    adapter_root().map(|root| root.join("node_modules/.bin"))
}

/// The PATH Harbor searches. Detection and launch must agree on it, or an
/// engine can be found and then fail to start.
pub fn runtime_path() -> String {
    let mut entries = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(bin) = adapter_bin_dir().filter(|bin| bin.is_dir()) {
        entries.insert(0, bin);
    }
    // The login shell is the authority. The fixed candidates below stay as a
    // fallback for the case where the probe cannot run at all.
    if let Some(probed) = login_shell_path() {
        for entry in env::split_paths(probed) {
            if entry.is_absolute() && !entries.iter().any(|existing| existing == &entry) {
                entries.push(entry);
            }
        }
    }
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

/// Fetch the ACP adapter an engine needs into Harbor's own directory. Nothing
/// is installed globally and nothing runs during the install but the package
/// manager itself, resolved from an absolute PATH entry like every other
/// executable Harbor starts.
pub fn install_adapter(engine_id: &str) -> Result<PathBuf, String> {
    let spec = catalog()
        .into_iter()
        .find(|spec| spec.id == engine_id)
        .ok_or_else(|| format!("unknown engine {engine_id}"))?;
    let package = spec
        .adapter_package
        .clone()
        .ok_or_else(|| format!("{} needs no adapter", spec.display_name))?;
    let binary = spec
        .binaries
        .get(1)
        .cloned()
        .ok_or_else(|| format!("{} names no adapter binary", spec.display_name))?;
    let root = adapter_root()
        .cloned()
        .ok_or_else(|| "adapter directory unavailable".to_string())?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    let path = runtime_path();
    let npm = resolve_on_path("npm", &path, None)
        .ok_or_else(|| "npm was not found. Install Node.js, then try again.".to_string())?;
    let output = Command::new(&npm)
        .args([
            "install",
            "--prefix",
            &root.display().to_string(),
            "--no-audit",
            "--no-fund",
            "--loglevel",
            "error",
            &package,
        ])
        .env("PATH", &path)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("npm could not be started. {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.lines().next_back().unwrap_or("npm failed").trim();
        return Err(format!("{package} could not be installed. {detail}"));
    }
    resolve_on_path(&binary, &runtime_path(), None)
        .ok_or_else(|| format!("{package} installed without a {binary} command"))
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
        adapter_package: (status == "adapter-missing")
            .then(|| spec.adapter_package.clone())
            .flatten(),
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
    fn login_shell_must_be_absolute_and_registered() {
        let registered = vec![PathBuf::from("/bin/zsh"), PathBuf::from("/bin/bash")];

        assert_eq!(
            login_shell(Some(Path::new("/bin/zsh")), &registered),
            Some(PathBuf::from("/bin/zsh"))
        );
        // A poisoned SHELL must not become a program Harbor runs.
        assert_eq!(login_shell(Some(Path::new("/tmp/evil")), &registered), None);
        assert_eq!(login_shell(Some(Path::new("zsh")), &registered), None);
        assert_eq!(login_shell(None, &registered), None);
        assert_eq!(login_shell(Some(Path::new("/bin/zsh")), &[]), None);
    }

    #[test]
    fn probe_output_is_read_between_sentinels_not_as_whole_output() {
        // Login rc files print banners; only the framed value counts.
        let noisy = format!(
            "Welcome back!\nnvm: using v22\n{PROBE_OPEN}/usr/local/bin:/usr/bin{PROBE_CLOSE}\nmotd\n"
        );
        assert_eq!(
            parse_probe(&noisy).as_deref(),
            Some("/usr/local/bin:/usr/bin")
        );

        assert_eq!(parse_probe("no sentinels here"), None);
        assert_eq!(parse_probe(&format!("{PROBE_OPEN}unterminated")), None);
        assert_eq!(parse_probe(&format!("{PROBE_OPEN}   {PROBE_CLOSE}")), None);
    }

    #[test]
    fn registered_shells_skips_comments_and_blanks() {
        // /etc/shells is the registry the guard consults; parsing it wrong would
        // either reject every shell or accept a commented-out line.
        let text = "# List of shells\n\n/bin/zsh\n  /bin/bash  \n#/bin/evil\n";
        let parsed: Vec<PathBuf> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(PathBuf::from)
            .collect();
        assert_eq!(
            parsed,
            vec![PathBuf::from("/bin/zsh"), PathBuf::from("/bin/bash")]
        );
        assert!(!parsed.contains(&PathBuf::from("/bin/evil")));
    }

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
        // The gap is nameable, so the view can offer to close it rather than
        // hiding the engine as if it were not installed at all.
        assert_eq!(
            engine.adapter_package.as_deref(),
            Some("@agentclientprotocol/claude-agent-acp@^0.75")
        );

        // A ready engine has nothing to install.
        let adapter = root.path().join("claude-agent-acp");
        fs::write(&adapter, "fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&adapter, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let engine = detect_engines(&root.path().display().to_string())
            .into_iter()
            .find(|engine| engine.id == "claude-code")
            .unwrap();
        assert_eq!(engine.status, "ready");
        assert!(engine.supports_chat);
        assert_eq!(engine.adapter_package, None);
    }

    #[test]
    fn install_adapter_refuses_engines_that_need_none() {
        assert!(install_adapter("opencode").is_err());
        assert!(install_adapter("nonesuch").is_err());
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
