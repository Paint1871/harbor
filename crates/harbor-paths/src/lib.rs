use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("path would expand in a shell")]
    Expandable,
    #[error("path escapes the granted root")]
    Escape,
    #[error("path is not absolute")]
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Unix,
    PowerShell,
}

pub fn refuse_if_expandable(raw: &str) -> Result<(), PathError> {
    if raw.chars().any(|ch| {
        matches!(
            ch,
            '$' | '`' | '*' | '?' | '!' | '\n' | ';' | '|' | '&' | '(' | ')' | '{' | '}'
        )
    }) {
        return Err(PathError::Expandable);
    }
    Ok(())
}

pub fn quote_for_shell(path: &Path, shell: ShellKind) -> Result<String, PathError> {
    let raw = path.to_str().ok_or(PathError::Expandable)?;
    refuse_if_expandable(raw)?;
    Ok(match shell {
        ShellKind::Unix => format!("'{}'", raw.replace('\'', "'\\''")),
        ShellKind::PowerShell => format!("'{}'", raw.replace('\'', "''")),
    })
}

pub fn assert_within(root: &Path, candidate: &Path) -> Result<PathBuf, PathError> {
    if !candidate.is_absolute() {
        return Err(PathError::Relative);
    }
    let root = root.canonicalize().map_err(|_| PathError::Escape)?;
    let resolved = candidate.canonicalize().map_err(|_| PathError::Escape)?;
    if resolved.starts_with(&root) {
        Ok(resolved)
    } else {
        Err(PathError::Escape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_expandable_and_quotes_safe_paths() {
        assert_eq!(refuse_if_expandable("$HOME/x"), Err(PathError::Expandable));
        assert!(refuse_if_expandable("/Users/me/proj").is_ok());
        let quoted = quote_for_shell(Path::new("/tmp/file"), ShellKind::Unix).unwrap();
        assert_eq!(quoted, "'/tmp/file'");
    }
}
