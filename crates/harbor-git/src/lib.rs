use std::io;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub patch: String,
}

/// Unified diff per changed file in a workspace folder.
pub fn unified_diffs(folder: &str) -> io::Result<Vec<FileDiff>> {
    let output = Command::new("git")
        .args(["-C", folder, "diff", "--no-color", "--", "."])
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    Ok(split_diffs(&String::from_utf8_lossy(&output.stdout)))
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
}
