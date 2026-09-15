//! Minimal stdio MCP server. Tokens stay in this process, never on the wire.

use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

/// Same name the ACP `SpawnSpec` puts on the sidecar. Session only; never a token.
pub const SESSION_ENV: &str = "HARBOR_PLUGIN_SESSION";

/// Resolve `--session <id>` (or `--session=`) then the sidecar env. Never reads
/// token-bearing variables.
pub fn session_ref<I, S>(args: I, env_session: Option<&str>) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let arg = arg.as_ref();
        if arg == "--session" {
            match args.next() {
                Some(value) if !value.as_ref().is_empty() => {
                    return Some(value.as_ref().to_string());
                }
                _ => break,
            }
        }
        if let Some(value) = arg.strip_prefix("--session=")
            && !value.is_empty()
        {
            return Some(value.to_string());
        }
    }
    env_session
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn protocol_version(request: &Value) -> &str {
    request
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .filter(|version| !version.is_empty())
        .unwrap_or("2024-11-05")
}

/// Answer one JSON-RPC message. Notifications (no `id`) return `None`.
pub fn handle_message(message: &Value) -> Option<Value> {
    let method = message.get("method")?.as_str()?;
    let id = message.get("id").cloned();
    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id?,
            "result": {
                "protocolVersion": protocol_version(message),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "harbor-plugins",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id?,
            "result": { "tools": [] }
        })),
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id?,
            "result": {}
        })),
        "notifications/initialized" | "initialized" => None,
        _ => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            })
        }),
    }
}

/// Newline-delimited JSON-RPC on stdio. Never echoes the request; never prints secrets.
pub fn serve<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(response) = handle_message(&message) else {
            continue;
        };
        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
}

/// `harbor mcp-plugins --session <id>`: answer MCP on stdio and exit. No window.
pub fn run() -> i32 {
    // Accepted so the engine can pass the Harbor session. Unused while tools
    // are empty. Must never be written to stdout or stderr.
    let _session = session_ref(std::env::args(), std::env::var(SESSION_ENV).ok().as_deref());
    match serve(io::stdin().lock(), io::stdout()) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looks_tokenish(value: &str) -> bool {
        value.contains("ghu_")
            || value.contains("gho_")
            || value.contains("ghp_")
            || value.contains("github_pat_")
            || value.contains("sk-")
            || value.contains("Bearer ")
    }

    #[test]
    fn session_flag_wins_over_env_and_ignores_token_names() {
        assert_eq!(
            session_ref(
                ["harbor", "mcp-plugins", "--session", "thread-1"],
                Some("ghu_should_not_win")
            )
            .as_deref(),
            Some("thread-1")
        );
        assert_eq!(
            session_ref(["harbor", "mcp-plugins", "--session=chat-2"], None).as_deref(),
            Some("chat-2")
        );
        assert_eq!(
            session_ref(["harbor", "mcp-plugins"], Some("sess-env")).as_deref(),
            Some("sess-env")
        );
        assert_eq!(
            session_ref(["harbor", "mcp-plugins"], Some("")).as_deref(),
            None
        );
        assert_eq!(
            session_ref(["harbor", "mcp-plugins"], None).as_deref(),
            None
        );
    }

    #[test]
    fn initialize_handshake_writes_a_result_on_stdout() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"engine","version":"0"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            "\n"
        );
        let mut out = Vec::new();
        serve(input.as_bytes(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!looks_tokenish(&text));
        let mut lines = text.lines().filter(|line| !line.is_empty());
        let init: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(init["jsonrpc"], "2.0");
        assert_eq!(init["id"], 1);
        assert!(init.get("error").is_none());
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(init["result"]["serverInfo"]["name"], "harbor-plugins");
        assert_eq!(
            init["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
        let tools: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(tools["id"], 2);
        assert_eq!(tools["result"]["tools"], json!([]));
        assert!(lines.next().is_none(), "initialized is a notification");
    }
}
