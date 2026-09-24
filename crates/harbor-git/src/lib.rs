use harbor_paths::assert_within;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path};
use std::process::{Command, Stdio};

/// Skip dumping untracked payloads larger than this; the panel still lists the path.
const UNTRACKED_CONTENT_MAX: usize = 64 * 1024;
/// The changes panel is a preview: cap the whole `git diff` payload instead of
/// buffering an unbounded stream into memory.
const DIFF_OUTPUT_MAX: usize = 4 * 1024 * 1024;
/// `git status --porcelain` is just paths, but bound it anyway.
const STATUS_OUTPUT_MAX: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub patch: String,
}

/// `git` resolves through well-known install locations before PATH: a
/// user-writable PATH entry planted by a hostile tool must not win a command
/// that reads and writes the workspace. Unknown layouts (NixOS, custom
/// prefixes) still fall back to PATH so git keeps working there.
#[cfg(not(windows))]
fn git_command() -> Command {
    const KNOWN: &[&str] = &[
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
        "/bin/git",
    ];
    for path in KNOWN {
        if Path::new(path).is_file() {
            return Command::new(path);
        }
    }
    Command::new("git")
}

/// The official Git for Windows installer lands in %ProgramFiles%\Git.
#[cfg(windows)]
fn git_command() -> Command {
    let program_files = std::env::var_os("ProgramFiles")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Program Files"));
    let candidate = program_files.join(r"Git\cmd\git.exe");
    if candidate.is_file() {
        Command::new(candidate)
    } else {
        Command::new("git")
    }
}

/// Run git with piped, capped stdout. Returns the output plus whether it was
/// truncated; `Ok(None)` means git failed without producing usable output.
fn git_output(folder: &str, args: &[&str], limit: usize) -> io::Result<Option<(Vec<u8>, bool)>> {
    let mut child = git_command()
        .arg("-C")
        .arg("-C")
        .arg(folder)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut buf = Vec::new();
    child
        .stdout
        .take()
        .expect("stdout is piped")
        .take(limit as u64 + 1)
        .read_to_end(&mut buf)?;
    let truncated = buf.len() > limit;
    if truncated {
        buf.truncate(limit);
        // Do not drain the rest — the process is no longer useful.
        let _ = child.kill();
    }
    let status = child.wait()?;
    if !status.success() && !truncated {
        return Ok(None);
    }
    Ok(Some((buf, truncated)))
}

/// Unified diff per changed file in a workspace folder, plus untracked paths.
/// Staged and unstaged changes are both included; external diff drivers and
/// textconv are disabled so a repo config cannot run arbitrary commands here.
pub fn unified_diffs(folder: &str) -> io::Result<Vec<FileDiff>> {
    let diff_args = [
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "HEAD",
        "--",
        ".",
    ];
    let mut diffs = match git_output(folder, &diff_args, DIFF_OUTPUT_MAX)? {
        Some((raw, truncated)) => {
            let mut diffs = split_diffs(&String::from_utf8_lossy(&raw));
            if truncated && let Some(last) = diffs.last_mut() {
                last.patch
                    .push_str("\n… truncated — the diff exceeds the preview limit …\n");
            }
            diffs
        }
        // A repository without commits has no HEAD; its index is the diff.
        None => git_output(
            folder,
            &[
                "diff",
                "--no-color",
                "--no-ext-diff",
                "--no-textconv",
                "--cached",
                "--",
                ".",
            ],
            DIFF_OUTPUT_MAX,
        )?
        .map(|(raw, _)| split_diffs(&String::from_utf8_lossy(&raw)))
        .unwrap_or_default(),
    };
    diffs.extend(collect_untracked_diffs(folder)?);
    Ok(diffs)
}

fn split_diffs(raw: &str) -> Vec<FileDiff> {
    let mut diffs = Vec::new();
    let mut current_path = String::new();
    let mut current = String::new();
    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("diff --git a/") {
            if !current_path.is_empty() {
                diffs.push(FileDiff {
                    path: current_path,
                    patch: std::mem::take(&mut current),
                });
            }
            current_path = path.split(" b/").next().unwrap_or(path).to_string();
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current_path.is_empty() {
        diffs.push(FileDiff {
            path: current_path,
            patch: current,
        });
    }
    diffs
}

fn collect_untracked_diffs(folder: &str) -> io::Result<Vec<FileDiff>> {
    let Some((raw, _)) = git_output(
        folder,
        &["status", "--porcelain", "--untracked-files=all"],
        STATUS_OUTPUT_MAX,
    )?
    else {
        return Ok(vec![]);
    };
    Ok(untracked_diffs(&String::from_utf8_lossy(&raw))
        .into_iter()
        .map(|diff| {
            let preview = read_untracked_preview(folder, &diff.path);
            FileDiff {
                patch: untracked_patch(&diff.path, preview.as_deref()),
                path: diff.path,
            }
        })
        .collect())
}

/// Build `FileDiff`s from `git status --porcelain` (or `-z`) output.
fn untracked_diffs(raw: &str) -> Vec<FileDiff> {
    untracked_paths(raw)
        .into_iter()
        .map(|path| FileDiff {
            patch: untracked_patch(&path, None),
            path,
        })
        .collect()
}

fn untracked_paths(raw: &str) -> Vec<String> {
    porcelain_records(raw)
        .into_iter()
        .filter_map(untracked_path_from_record)
        .collect()
}

fn porcelain_records(raw: &str) -> Vec<&str> {
    if raw.contains('\0') {
        raw.split('\0')
            .filter(|record| !record.is_empty())
            .collect()
    } else {
        raw.lines().collect()
    }
}

fn untracked_path_from_record(record: &str) -> Option<String> {
    let rest = record.strip_prefix("?? ")?;
    let path = unquote_git_path(rest);
    if path.is_empty() { None } else { Some(path) }
}

fn unquote_git_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(inner) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return trimmed.to_string();
    };
    unescape_git_path(inner)
}

fn unescape_git_path(inner: &str) -> String {
    let mut out = Vec::with_capacity(inner.len());
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        match bytes.get(i) {
            Some(b'\\') => {
                out.push(b'\\');
                i += 1;
            }
            Some(b'"') => {
                out.push(b'"');
                i += 1;
            }
            Some(b'n') => {
                out.push(b'\n');
                i += 1;
            }
            Some(b't') => {
                out.push(b'\t');
                i += 1;
            }
            Some(b'r') => {
                out.push(b'\r');
                i += 1;
            }
            Some(d) if (b'0'..=b'7').contains(d) => {
                let mut val = 0u8;
                let mut count = 0;
                while count < 3 {
                    match bytes.get(i) {
                        Some(n) if (b'0'..=b'7').contains(n) => {
                            val = val.saturating_mul(8).saturating_add(n - b'0');
                            i += 1;
                            count += 1;
                        }
                        _ => break,
                    }
                }
                out.push(val);
            }
            Some(other) => {
                out.push(b'\\');
                out.push(*other);
                i += 1;
            }
            None => out.push(b'\\'),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn untracked_patch(path: &str, contents: Option<&[u8]>) -> String {
    let mut patch = format!("untracked\nnew file: {path}\n");
    let Some(bytes) = contents else {
        return patch;
    };
    if bytes.len() > UNTRACKED_CONTENT_MAX || bytes.contains(&0) {
        return patch;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return patch;
    };
    if text.is_empty() {
        return patch;
    }
    patch.push('\n');
    patch.push_str(text);
    if !text.ends_with('\n') {
        patch.push('\n');
    }
    patch
}

fn read_untracked_preview(folder: &str, relative: &str) -> Option<Vec<u8>> {
    let relative_path = Path::new(relative);
    // The path list comes from `git status`, but never trust it blindly: no
    // absolute paths and no `..` escapes.
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return None;
    }
    let path = Path::new(folder).join(relative_path);
    // A symlink's target may live anywhere on disk — the preview must not
    // follow it. `symlink_metadata` inspects the link itself, not the target.
    let meta = fs::symlink_metadata(&path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > UNTRACKED_CONTENT_MAX as u64
    {
        return None;
    }
    // A parent directory may itself be a link out of the workspace; resolve
    // and read the canonical path so the content provably stays inside.
    let resolved = assert_within(Path::new(folder), &path).ok()?;
    fs::read(resolved).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_unified_diff_per_file() {
        let raw =
            "diff --git a/src/a.rs b/src/a.rs\n+one\ndiff --git a/src/b.rs b/src/b.rs\n+two\n";
        let diffs = split_diffs(raw);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].path, "src/a.rs");
        assert!(diffs[1].patch.contains("+two"));
    }

    #[test]
    fn untracked_porcelain_lists_only_untracked_paths() {
        let raw = concat!(
            " M src/a.rs\n",
            "?? src/new.rs\n",
            "A  staged.rs\n",
            "?? README.md\n",
            "R  old.rs -> renamed.rs\n",
        );
        let diffs = untracked_diffs(raw);
        assert_eq!(
            diffs
                .iter()
                .map(|diff| diff.path.as_str())
                .collect::<Vec<_>>(),
            ["src/new.rs", "README.md"]
        );
        assert!(diffs[0].patch.contains("untracked"));
        assert!(diffs[0].patch.contains("new file: src/new.rs"));
        assert!(diffs[1].patch.contains("new file: README.md"));
        assert!(!diffs[0].patch.contains("staged.rs"));
    }

    #[test]
    fn untracked_porcelain_unquotes_paths() {
        let raw = "?? \"foo bar.txt\"\n?? \"nested/quote\\\"d.rs\"\n?? \"caf\\303\\251.txt\"\n";
        let diffs = untracked_diffs(raw);
        assert_eq!(diffs[0].path, "foo bar.txt");
        assert_eq!(diffs[1].path, "nested/quote\"d.rs");
        assert_eq!(diffs[2].path, "café.txt");
        assert!(diffs[0].patch.contains("new file: foo bar.txt"));
    }

    #[test]
    fn untracked_porcelain_accepts_nul_records() {
        let raw = " M src/a.rs\0?? src/new.rs\0?? notes.md\0";
        let diffs = untracked_diffs(raw);
        assert_eq!(
            diffs
                .iter()
                .map(|diff| diff.path.as_str())
                .collect::<Vec<_>>(),
            ["src/new.rs", "notes.md"]
        );
    }

    #[test]
    fn untracked_patch_skips_binary_and_huge_contents() {
        let small = untracked_patch("notes.md", Some(b"hello\n"));
        assert!(small.contains("untracked"));
        assert!(small.contains("new file: notes.md"));
        assert!(small.contains("hello"));

        let binary = untracked_patch("blob.bin", Some(&[b'a', 0, b'b']));
        assert_eq!(binary, "untracked\nnew file: blob.bin\n");

        let huge = vec![b'x'; UNTRACKED_CONTENT_MAX + 1];
        let skipped = untracked_patch("huge.md", Some(&huge));
        assert_eq!(skipped, "untracked\nnew file: huge.md\n");
        assert!(!skipped.contains("xxx"));
    }

    #[test]
    fn untracked_preview_reads_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.md"), b"hi").unwrap();
        let folder = dir.path().to_str().unwrap();
        assert_eq!(
            read_untracked_preview(folder, "notes.md"),
            Some(b"hi".to_vec())
        );
    }

    #[test]
    fn untracked_preview_refuses_dotdot_and_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let folder = dir.path().to_str().unwrap();
        assert_eq!(read_untracked_preview(folder, "../escape"), None);
        assert_eq!(read_untracked_preview(folder, "a/../../escape"), None);
        assert_eq!(read_untracked_preview(folder, "/etc/hosts"), None);
    }

    #[cfg(unix)]
    #[test]
    fn untracked_preview_refuses_a_symlink_out_of_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"secret").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link.txt")).unwrap();
        let folder = dir.path().to_str().unwrap();
        assert_eq!(read_untracked_preview(folder, "link.txt"), None);
    }

    #[cfg(unix)]
    #[test]
    fn untracked_preview_refuses_a_symlinked_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), b"secret").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).unwrap();
        let folder = dir.path().to_str().unwrap();
        assert_eq!(read_untracked_preview(folder, "sub/secret.txt"), None);
    }
}
