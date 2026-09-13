//! What the hosted CLIs say about their own rate limits.
//!
//! Harbor has no account and no backend, so it never asks a vendor how much of
//! a plan is left. Every number here was written on this machine by the CLI
//! itself: Codex logs a snapshot beside each session, and Claude Code hands one
//! to its status line, which Harbor's bridge mirrors into a file.
//!
//! An engine therefore appears only while it publishes its limits locally, and
//! a window whose reset has passed is dropped rather than shown as a fresh zero
//! — stale numbers under a full bar are worse than no bar.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// How many of the newest rollout files to open before giving up. A session
/// that never reached the API writes no limits, and a day boundary splits the
/// newest ones across two directories.
const CANDIDATES: usize = 8;

/// A long session's rollout runs to megabytes and the snapshot we want is the
/// last one in it, so only the tail is read.
const TAIL_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    /// The length of the window in minutes; 300 is the five-hour window.
    pub window_minutes: i64,
    pub used_percent: f64,
    /// Unix seconds at which this window rolls over.
    pub resets_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineUsage {
    pub engine_id: String,
    pub display_name: String,
    pub windows: Vec<UsageWindow>,
    /// Unix seconds: when the engine last wrote these numbers down.
    pub measured_at: i64,
}

/// The file the status-line bridge writes for Claude Code, inside `usage_dir`.
pub const CLAUDE_CODE_CACHE: &str = "claude-code.json";

/// Every engine that published usable limits, newest reading each. `usage_dir`
/// is where Harbor keeps what its own bridges captured.
pub fn read(usage_dir: &Path, now: i64) -> Vec<EngineUsage> {
    let mut engines = Vec::new();
    if let Some(usage) = claude_code(&usage_dir.join(CLAUDE_CODE_CACHE), now) {
        engines.push(usage);
    }
    if let Some(usage) = home().and_then(|home| codex(&home.join(".codex/sessions"), now)) {
        engines.push(usage);
    }
    engines
}

/// Claude Code reports `five_hour`, `seven_day` and, behind a gateway,
/// `spend_limit`. Only the first two are a subscription window a builder can
/// pace against, so the spend limit is left to the gateway that owns it.
fn claude_code(cache: &Path, now: i64) -> Option<EngineUsage> {
    let text = std::fs::read_to_string(cache).ok()?;
    let cached: Value = serde_json::from_str(&text).ok()?;
    let limits = cached.get("rateLimits")?;
    let windows: Vec<UsageWindow> = [("five_hour", 300), ("seven_day", 10_080)]
        .iter()
        .filter_map(|(slot, minutes)| {
            let slot = limits.get(slot)?;
            Some(UsageWindow {
                window_minutes: *minutes,
                used_percent: slot.get("used_percentage")?.as_f64()?,
                resets_at: slot.get("resets_at")?.as_i64()?,
            })
        })
        .filter(|window| window.resets_at > now)
        .collect();
    if windows.is_empty() {
        return None;
    }
    Some(EngineUsage {
        engine_id: "claude-code".into(),
        display_name: display_name("claude-code"),
        windows,
        measured_at: cached
            .get("measuredAt")
            .and_then(Value::as_i64)
            .unwrap_or_else(|| modified_at(cache)),
    })
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Codex records a `rate_limits` snapshot with every token count it logs, so
/// the last one in the newest rollout is the most recent reading on this
/// machine.
fn codex(sessions: &Path, now: i64) -> Option<EngineUsage> {
    for path in newest_rollouts(sessions) {
        let Some(text) = tail(&path) else {
            continue;
        };
        let Some(limits) = text
            .lines()
            .rev()
            .filter(|line| line.contains("\"rate_limits\""))
            .find_map(|line| serde_json::from_str::<Value>(line).ok())
            .and_then(|line| find_key(&line, "rate_limits").cloned())
        else {
            continue;
        };
        let windows: Vec<UsageWindow> = ["primary", "secondary"]
            .iter()
            .filter_map(|slot| window(limits.get(slot)?))
            .filter(|window| window.resets_at > now)
            .collect();
        if windows.is_empty() {
            continue;
        }
        return Some(EngineUsage {
            engine_id: "codex".into(),
            display_name: display_name("codex"),
            windows,
            measured_at: modified_at(&path),
        });
    }
    None
}

/// The catalog's name for the engine, so the strip reads `Codex`, not `codex`.
fn display_name(engine_id: &str) -> String {
    crate::engines::catalog()
        .into_iter()
        .find(|spec| spec.id == engine_id)
        .map(|spec| spec.display_name)
        .unwrap_or_else(|| engine_id.to_string())
}

/// The last `TAIL_BYTES` of the file, minus the partial line the cut lands in.
fn tail(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let from = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(from)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if from == 0 {
        return Some(text);
    }
    text.find('\n').map(|cut| text[cut + 1..].to_string())
}

fn window(slot: &Value) -> Option<UsageWindow> {
    Some(UsageWindow {
        window_minutes: slot.get("window_minutes")?.as_i64()?,
        used_percent: slot.get("used_percent")?.as_f64()?,
        resets_at: slot.get("resets_at")?.as_i64()?,
    })
}

fn find_key<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => {
            if let Some(found) = fields.get(key) {
                return Some(found);
            }
            fields.values().find_map(|field| find_key(field, key))
        }
        Value::Array(items) => items.iter().find_map(|item| find_key(item, key)),
        _ => None,
    }
}

fn modified_at(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

/// Rollouts live under `sessions/<year>/<month>/<day>/` and their names begin
/// with an ISO timestamp, so descending the highest-named directory first walks
/// them newest to oldest without stating every file in the tree.
fn newest_rollouts(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    descend(root, &mut found);
    found
}

fn descend(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let (mut dirs, mut files): (Vec<PathBuf>, Vec<PathBuf>) = (Vec::new(), Vec::new());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
    files.sort();
    for file in files.into_iter().rev() {
        if found.len() >= CANDIDATES {
            return;
        }
        found.push(file);
    }
    dirs.sort();
    for dir in dirs.into_iter().rev() {
        if found.len() >= CANDIDATES {
            return;
        }
        descend(&dir, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rollout(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(name), body).unwrap();
    }

    fn snapshot(primary: (f64, i64), secondary: (f64, i64)) -> String {
        format!(
            r#"{{"type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"limit_id":"codex","primary":{{"used_percent":{},"window_minutes":300,"resets_at":{}}},"secondary":{{"used_percent":{},"window_minutes":10080,"resets_at":{}}}}}}}}}"#,
            primary.0, primary.1, secondary.0, secondary.1
        )
    }

    #[test]
    fn reads_the_last_snapshot_from_the_newest_rollout() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        rollout(
            &sessions.join("2026/09/06"),
            "rollout-2026-09-06T02-37-20-old.jsonl",
            &snapshot((99.0, 5_000), (99.0, 9_000)),
        );
        let newest = sessions.join("2026/09/07");
        rollout(
            &newest,
            "rollout-2026-09-07T04-51-00-new.jsonl",
            &format!(
                "{}\n{}\n{}\n",
                snapshot((3.0, 5_000), (12.0, 9_000)),
                r#"{"type":"event_msg","payload":{"type":"agent_message"}}"#,
                snapshot((36.0, 5_000), (14.0, 9_000)),
            ),
        );

        let usage = codex(&sessions, 1_000).expect("usage");
        assert_eq!(usage.engine_id, "codex");
        assert_eq!(
            usage.windows,
            vec![
                UsageWindow {
                    window_minutes: 300,
                    used_percent: 36.0,
                    resets_at: 5_000
                },
                UsageWindow {
                    window_minutes: 10080,
                    used_percent: 14.0,
                    resets_at: 9_000
                },
            ]
        );
    }

    #[test]
    fn a_window_that_has_already_reset_is_not_reported_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        rollout(
            &sessions.join("2026/09/07"),
            "rollout-2026-09-07T04-51-00-new.jsonl",
            &snapshot((36.0, 5_000), (14.0, 9_000)),
        );

        let usage = codex(&sessions, 6_000).expect("usage");
        assert_eq!(usage.windows.len(), 1);
        assert_eq!(usage.windows[0].window_minutes, 10080);

        assert!(codex(&sessions, 10_000).is_none());
    }

    #[test]
    fn a_session_without_limits_falls_through_to_one_that_has_them() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        rollout(
            &sessions.join("2026/09/06"),
            "rollout-2026-09-06T02-37-20-old.jsonl",
            &snapshot((36.0, 5_000), (14.0, 9_000)),
        );
        rollout(
            &sessions.join("2026/09/07"),
            "rollout-2026-09-07T04-51-00-quiet.jsonl",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"agent_message\"}}\n",
        );

        let usage = codex(&sessions, 1_000).expect("usage");
        assert_eq!(usage.windows[0].used_percent, 36.0);
    }

    #[test]
    fn only_the_tail_is_read_and_a_cut_line_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        let padding = format!(
            r#"{{"type":"event_msg","filler":"{}"}}"#,
            "x".repeat(TAIL_BYTES as usize)
        );
        rollout(
            &sessions.join("2026/09/07"),
            "rollout-2026-09-07T04-51-00-long.jsonl",
            &format!("{padding}\n{}\n", snapshot((36.0, 5_000), (14.0, 9_000))),
        );

        let usage = codex(&sessions, 1_000).expect("usage");
        assert_eq!(usage.display_name, "Codex");
        assert_eq!(usage.windows[0].used_percent, 36.0);
    }

    #[test]
    fn a_missing_directory_is_not_an_error() {
        assert!(codex(Path::new("/harbor/does/not/exist"), 0).is_none());
    }
}
