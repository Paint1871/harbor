use std::path::{Path, PathBuf};

use harbor_paths::{ShellKind, quote_for_shell, refuse_if_expandable};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PtyError {
    #[error("executable path must be absolute")]
    Relative,
    #[error("executable is not on the host allowlist")]
    Denied,
    #[error(transparent)]
    Path(#[from] harbor_paths::PathError),
}

#[derive(Debug, Clone)]
pub struct PtySpawn {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Spawn request for later portable-pty wiring. Only absolute allowlisted programs.
pub fn prepare_spawn(
    program: &Path,
    cwd: &Path,
    allowlisted: &[PathBuf],
) -> Result<PtySpawn, PtyError> {
    if !program.is_absolute() {
        return Err(PtyError::Relative);
    }
    if !allowlisted.iter().any(|granted| granted == program) {
        return Err(PtyError::Denied);
    }
    Ok(PtySpawn {
        program: program.to_path_buf(),
        args: Vec::new(),
        cwd: cwd.to_path_buf(),
    })
}

pub fn typed_path(raw: &str, unix: bool) -> Result<String, PtyError> {
    refuse_if_expandable(raw)?;
    Ok(quote_for_shell(
        Path::new(raw),
        if unix {
            ShellKind::Unix
        } else {
            ShellKind::PowerShell
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_relative_and_unlisted_programs() {
        let allow = vec![PathBuf::from("/bin/zsh")];
        assert_eq!(
            prepare_spawn(Path::new("zsh"), Path::new("/tmp"), &allow).unwrap_err(),
            PtyError::Relative
        );
        assert_eq!(
            prepare_spawn(Path::new("/bin/bash"), Path::new("/tmp"), &allow).unwrap_err(),
            PtyError::Denied
        );
        assert!(prepare_spawn(Path::new("/bin/zsh"), Path::new("/tmp"), &allow).is_ok());
    }

    #[test]
    fn quoted_paths_refuse_expansion() {
        assert!(typed_path("$HOME/x", true).is_err());
        assert_eq!(typed_path("/tmp/file", true).unwrap(), "'/tmp/file'");
    }
}
