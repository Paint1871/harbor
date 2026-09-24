//! The status-line bridge for Claude Code.
//!
//! Claude Code knows its own rate limits but only ever hands them to a status
//! line: one JSON object on the command's stdin, whose stdout it displays. So
//! Harbor registers itself as that command, keeps the `rate_limits` block, and
//! then runs whatever command was configured before, printing its output
//! unchanged. Connecting must not cost the builder the status line they had.
//!
//! The previous command is remembered in Harbor's own directory rather than
//! wrapped into the settings entry, so nothing has to survive a second round of
//! shell quoting and disconnecting is just putting it back.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// What Harbor remembers about the status line it replaced.
const BRIDGE_STATE: &str = "bridge.json";
/// The untouched settings file, kept before Harbor first writes to it.
const SETTINGS_BACKUP: &str = "claude-settings.backup.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub connected: bool,
    /// The status line Harbor runs after its own, when there was one.
    pub chained: Option<String>,
    /// Absolute path to the settings file Harbor would edit.
    pub settings_path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BridgeState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chained: Option<String>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn settings_path() -> Option<PathBuf> {
    Some(home()?.join(".claude/settings.json"))
}

/// The status-line command Harbor installs. The executable is quoted because
/// Claude Code runs the entry as a shell line and Harbor's own path may well
/// contain a space.
fn bridge_command(exe: &Path) -> String {
    let raw = exe.to_string_lossy();
    if cfg!(windows) {
        format!("\"{raw}\" usage-bridge")
    } else {
        format!("'{}' usage-bridge", raw.replace('\'', "'\\''"))
    }
}

/// Harbor's own entry is exactly `'<exe>' usage-bridge`. A looser substring
/// match would eat a status line that merely mentions the word
/// (`~/bin/my-usage-bridge.py` is somebody's own script, not our bridge).
/// The quoted-path + trailing-token shape also recognizes entries written by
/// a Harbor exe at a different path (app moved between connect and now).
fn is_bridge(command: &str, exe: &Path) -> bool {
    let trimmed = command.trim();
    if trimmed == bridge_command(exe) {
        return true;
    }
    trimmed.ends_with(" usage-bridge") && (trimmed.starts_with('\'') || trimmed.starts_with('"'))
}

fn read_state(usage_dir: &Path) -> BridgeState {
    std::fs::read_to_string(usage_dir.join(BRIDGE_STATE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, format!("{text}\n")).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, path).map_err(|error| error.to_string())
}

fn read_settings(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| format!("{} is not valid JSON: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(error) => Err(error.to_string()),
    }
}

pub fn status(usage_dir: &Path, settings: &Path, exe: &Path) -> BridgeStatus {
    let connected = read_settings(settings)
        .ok()
        .and_then(|settings| {
            let line = settings.get("statusLine")?;
            Some(is_bridge(line.get("command")?.as_str()?, exe))
        })
        .unwrap_or(false);
    BridgeStatus {
        connected,
        chained: read_state(usage_dir).chained,
        settings_path: settings.display().to_string(),
    }
}

/// Points Claude Code's status line at Harbor, keeping a copy of the file as it
/// was and remembering any command it already ran.
pub fn connect(usage_dir: &Path, path: &Path, exe: &Path) -> Result<BridgeStatus, String> {
    let mut settings = read_settings(path)?;
    if !settings.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }

    if path.exists() {
        let backup = usage_dir.join(SETTINGS_BACKUP);
        if !backup.exists() {
            std::fs::create_dir_all(usage_dir).map_err(|error| error.to_string())?;
            std::fs::copy(path, &backup).map_err(|error| error.to_string())?;
        }
    }

    let previous = settings
        .get("statusLine")
        .and_then(|line| line.get("command"))
        .and_then(Value::as_str)
        .filter(|command| !is_bridge(command, exe))
        .map(str::to_string);
    if previous.is_some() {
        write_json(
            &usage_dir.join(BRIDGE_STATE),
            &serde_json::to_value(BridgeState {
                chained: previous.clone(),
            })
            .map_err(|error| error.to_string())?,
        )?;
    }

    settings["statusLine"] = json!({ "type": "command", "command": bridge_command(exe) });
    write_json(path, &settings)?;
    Ok(status(usage_dir, path, exe))
}

/// Puts the builder's own status line back, or removes the entry Harbor added.
/// If the settings file no longer parses at all, the untouched pre-connect
/// copy is the honest way back — that is what the backup exists for.
pub fn disconnect(usage_dir: &Path, path: &Path, exe: &Path) -> Result<BridgeStatus, String> {
    let mut settings = match read_settings(path) {
        Ok(settings) => settings,
        Err(error) => {
            let backup = usage_dir.join(SETTINGS_BACKUP);
            if !backup.is_file() {
                return Err(error);
            }
            std::fs::copy(&backup, path).map_err(|copy| copy.to_string())?;
            read_settings(path)?
        }
    };
    let ours = settings
        .get("statusLine")
        .and_then(|line| line.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| is_bridge(command, exe));
    if ours {
        match read_state(usage_dir).chained {
            Some(command) => {
                settings["statusLine"] = json!({ "type": "command", "command": command });
            }
            None => {
                if let Some(fields) = settings.as_object_mut() {
                    fields.remove("statusLine");
                }
            }
        }
        write_json(path, &settings)?;
    }
    let _ = std::fs::remove_file(usage_dir.join(BRIDGE_STATE));
    let _ = std::fs::remove_file(usage_dir.join(harbor_core::usage::CLAUDE_CODE_CACHE));
    // Settings are healthy again, so the pre-connect snapshot goes stale; the
    // next connect takes a fresh one.
    let _ = std::fs::remove_file(usage_dir.join(SETTINGS_BACKUP));
    Ok(status(usage_dir, path, exe))
}

/// Runs as the status-line command: keep the limits, then hand the same stdin
/// to the command Harbor replaced and print what it prints.
pub fn run(usage_dir: &Path) -> i32 {
    let mut payload = String::new();
    if std::io::stdin().read_to_string(&mut payload).is_err() {
        return 0;
    }

    if let Some(limits) = serde_json::from_str::<Value>(&payload)
        .ok()
        .and_then(|value| value.get("rate_limits").cloned())
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        let _ = write_json(
            &usage_dir.join(harbor_core::usage::CLAUDE_CODE_CACHE),
            &json!({ "rateLimits": limits, "measuredAt": now }),
        );
    }

    let Some(chained) = read_state(usage_dir).chained else {
        return 0;
    };
    let (shell, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("/bin/sh", "-c")
    };
    let Ok(mut child) = Command::new(shell)
        .arg(flag)
        .arg(&chained)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())
        .spawn()
    else {
        return 0;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.as_bytes());
    }
    child.wait().map(|_| 0).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connecting_keeps_the_builders_own_status_line_and_gives_it_back() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join("usage");
        let settings = dir.path().join(".claude/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(
            &settings,
            r#"{"theme":"dark","statusLine":{"type":"command","command":"~/mine.sh"}}"#,
        )
        .unwrap();

        {
            let exe = Path::new("/opt/Harbor.app/harbor");
            let connected = connect(&usage, &settings, exe).unwrap();
            assert!(connected.connected);
            assert_eq!(connected.chained.as_deref(), Some("~/mine.sh"));

            let written: Value =
                serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
            assert_eq!(
                written["statusLine"]["command"],
                "'/opt/Harbor.app/harbor' usage-bridge"
            );
            // Everything else in the file survives the edit.
            assert_eq!(written["theme"], "dark");
            assert!(usage.join(SETTINGS_BACKUP).exists());

            let off = disconnect(&usage, &settings, exe).unwrap();
            assert!(!off.connected);
            let restored: Value =
                serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
            assert_eq!(restored["statusLine"]["command"], "~/mine.sh");
        }
    }

    #[test]
    fn disconnecting_removes_an_entry_harbor_added_itself() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join("usage");
        let settings = dir.path().join(".claude/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, r#"{"theme":"dark"}"#).unwrap();

        {
            let exe = Path::new("/opt/harbor");
            connect(&usage, &settings, exe).unwrap();
            disconnect(&usage, &settings, exe).unwrap();
            let after: Value =
                serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
            assert!(after.get("statusLine").is_none());
            assert_eq!(after["theme"], "dark");
        }
    }

    #[test]
    fn a_missing_settings_file_is_created_rather_than_refused() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join("usage");
        let settings = dir.path().join(".claude/settings.json");
        let exe = Path::new("/opt/harbor");
        assert!(!status(&usage, &settings, exe).connected);
        assert!(connect(&usage, &settings, exe).unwrap().connected);
        assert!(status(&usage, &settings, exe).connected);
    }

    #[test]
    fn a_command_that_mentions_usage_bridge_is_not_ours() {
        let exe = Path::new("/opt/harbor");
        // A substring match would have claimed somebody's own script.
        assert!(!is_bridge("python ~/bin/my-usage-bridge.py", exe));
        assert!(is_bridge("'/opt/harbor' usage-bridge", exe));
        // Entries from an exe at a different path still count as ours.
        assert!(is_bridge("'/opt/moved/harbor' usage-bridge", exe));
        assert!(!is_bridge("usage-bridge --alone", exe));
    }

    #[test]
    fn a_corrupt_settings_file_falls_back_to_the_backup() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join("usage");
        let settings = dir.path().join(".claude/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let exe = Path::new("/opt/harbor");

        std::fs::write(
            &settings,
            r#"{"theme":"dark","statusLine":{"type":"command","command":"~/mine.sh"}}"#,
        )
        .unwrap();
        connect(&usage, &settings, exe).unwrap();
        assert!(usage.join(SETTINGS_BACKUP).exists());

        // Whatever corrupted the file, disconnect leaves the pre-Harbor copy.
        std::fs::write(&settings, "{ not json").unwrap();
        let off = disconnect(&usage, &settings, exe).unwrap();
        assert!(!off.connected);
        let restored: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored["statusLine"]["command"], "~/mine.sh");
        assert!(!usage.join(SETTINGS_BACKUP).exists());
    }

    #[test]
    fn broken_json_is_reported_instead_of_being_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let usage = dir.path().join("usage");
        let settings = dir.path().join(".claude/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, "{ not json").unwrap();

        assert!(connect(&usage, &settings, Path::new("/opt/harbor")).is_err());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{ not json");
    }
}
