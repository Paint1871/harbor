use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    sync::RwLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableKind {
    LoginShell,
    Engine,
    Harbor,
}

/// Host-owned grants. Never expose registration as a renderer IPC command.
#[derive(Default)]
pub struct ExecutableAllowlist(RwLock<BTreeMap<PathBuf, ExecutableKind>>);

impl ExecutableAllowlist {
    pub fn bootstrap() -> io::Result<Self> {
        let allowlist = Self::default();
        allowlist.grant(&env::current_exe()?, ExecutableKind::Harbor)?;
        #[cfg(unix)]
        {
            let default = if cfg!(target_os = "macos") {
                "/bin/zsh"
            } else {
                "/bin/sh"
            };
            let shell = env::var_os("SHELL")
                .map(PathBuf::from)
                .unwrap_or_else(|| default.into());
            // An unavailable configured shell must not prevent the local window opening.
            let _ = allowlist.grant(&shell, ExecutableKind::LoginShell);
        }
        #[cfg(windows)]
        {
            if let Some(path) = env::var_os("PATH") {
                let cwd = env::current_dir()?;
                for name in ["powershell.exe", "pwsh.exe", "cmd.exe"] {
                    if let Some(shell) = resolve_on_path(name, &path, &cwd, &cwd) {
                        let _ = allowlist.grant(&shell, ExecutableKind::LoginShell);
                    }
                }
            }
            // wsl.exe is not granted until the user chooses a distro in Settings.
        }
        Ok(allowlist)
    }

    /// Only trusted host discovery/settings may grant a resolved shell or engine.
    pub fn grant(&self, path: &Path, kind: ExecutableKind) -> io::Result<PathBuf> {
        let resolved = executable_path(path)?;
        self.0
            .write()
            .map_err(|_| io::Error::other("executable grants unavailable"))?
            .insert(resolved.clone(), kind);
        Ok(resolved)
    }

    /// Every later PTY/ACP spawn must use this returned absolute path.
    pub fn authorize(&self, path: &Path, kind: ExecutableKind) -> io::Result<PathBuf> {
        let resolved = executable_path(path)?;
        let grants = self
            .0
            .read()
            .map_err(|_| io::Error::other("executable grants unavailable"))?;
        if grants.get(&resolved) == Some(&kind) {
            Ok(resolved)
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "executable is not granted for this role",
            ))
        }
    }

    pub fn granted(&self, kind: ExecutableKind) -> Vec<PathBuf> {
        self.0
            .read()
            .ok()
            .map(|grants| {
                grants
                    .iter()
                    .filter(|(_, granted)| **granted == kind)
                    .map(|(path, _)| path.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn executable_path(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "executable path must be absolute",
        ));
    }
    let resolved = path.canonicalize()?;
    let metadata = fs::metadata(&resolved)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "executable must be a file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "file is not executable",
            ));
        }
    }
    Ok(resolved)
}

/// Named Windows shells search absolute PATH entries only, never cwd or relative entries.
pub fn resolve_on_path(
    name: &str,
    search_path: &std::ffi::OsStr,
    workspace: &Path,
    cwd: &Path,
) -> Option<PathBuf> {
    if name.contains(['/', '\\']) || name == "." || name == ".." || name.is_empty() {
        return None;
    }
    let workspace = workspace.canonicalize().ok()?;
    let cwd = cwd.canonicalize().ok();
    env::split_paths(search_path)
        .filter(|directory| directory.is_absolute())
        .find_map(|directory| {
            let directory = directory.canonicalize().ok()?;
            if cwd.as_ref().is_some_and(|cwd| directory == *cwd) {
                return None;
            }
            if directory.starts_with(&workspace) {
                return None;
            }
            executable_path(&directory.join(name)).ok()
        })
}

pub fn allows_navigation(url: &tauri::Url, development: bool) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match (url.scheme(), url.host_str(), url.port()) {
        ("tauri", Some("localhost"), None) => true,
        ("http" | "https", Some("tauri.localhost"), None) => true,
        ("http", Some("127.0.0.1"), Some(1420)) if development => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executable(path: &Path) {
        fs::write(path, "fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[test]
    fn grants_are_absolute_and_role_specific() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("engine");
        let second = root.path().join("other");
        executable(&first);
        executable(&second);
        let list = ExecutableAllowlist::default();
        assert!(
            list.grant(Path::new("engine"), ExecutableKind::Engine)
                .is_err()
        );
        assert!(list.authorize(&first, ExecutableKind::Engine).is_err());
        list.grant(&first, ExecutableKind::Engine).unwrap();
        assert_eq!(
            list.authorize(&first, ExecutableKind::Engine).unwrap(),
            first.canonicalize().unwrap()
        );
        assert!(list.authorize(&first, ExecutableKind::LoginShell).is_err());
        assert!(list.authorize(&second, ExecutableKind::Engine).is_err());
        assert!(list.grant(root.path(), ExecutableKind::Engine).is_err());
    }

    #[test]
    fn path_search_excludes_cwd_workspace_and_relative_entries() {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("cwd");
        let workspace = root.path().join("workspace");
        let trusted = root.path().join("trusted");
        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&trusted).unwrap();
        executable(&cwd.join("shell.exe"));
        executable(&workspace.join("shell.exe"));
        executable(&trusted.join("shell.exe"));
        let paths = env::join_paths([&cwd, &workspace, Path::new("."), &trusted]).unwrap();
        assert_eq!(
            resolve_on_path("shell.exe", &paths, &workspace, &cwd),
            Some(trusted.join("shell.exe").canonicalize().unwrap())
        );
        assert!(resolve_on_path("../shell.exe", &paths, &workspace, &cwd).is_none());
    }

    #[test]
    fn bootstrap_grants_running_binary_as_harbor_not_engine() {
        let list = ExecutableAllowlist::bootstrap().unwrap();
        let exe = env::current_exe().unwrap();
        assert_eq!(
            list.authorize(&exe, ExecutableKind::Harbor).unwrap(),
            exe.canonicalize().unwrap()
        );
        assert!(list.authorize(&exe, ExecutableKind::Engine).is_err());
        #[cfg(unix)]
        if let Some(shell) = env::var_os("SHELL").map(PathBuf::from)
            && shell.is_absolute()
            && shell.exists()
        {
            assert!(
                list.authorize(&shell, ExecutableKind::LoginShell)
                    .unwrap()
                    .is_absolute()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn retargeted_symlink_does_not_inherit_grant() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        let link = root.path().join("link");
        executable(&first);
        executable(&second);
        symlink(&first, &link).unwrap();
        let list = ExecutableAllowlist::default();
        list.grant(&link, ExecutableKind::Engine).unwrap();
        fs::remove_file(&link).unwrap();
        symlink(&second, &link).unwrap();
        assert!(list.authorize(&link, ExecutableKind::Engine).is_err());
    }

    #[test]
    fn navigation_stays_local_and_dev_origin_is_not_in_release() {
        for value in [
            "https://example.com",
            "file:///etc/passwd",
            "https://tauri.localhost.example.com",
            "http://user@tauri.localhost",
        ] {
            assert!(!allows_navigation(&value.parse().unwrap(), true));
        }
        assert!(allows_navigation(
            &"tauri://localhost/index.html".parse().unwrap(),
            false
        ));
        let dev = "http://127.0.0.1:1420".parse().unwrap();
        assert!(allows_navigation(&dev, true));
        assert!(!allows_navigation(&dev, false));
    }
}
