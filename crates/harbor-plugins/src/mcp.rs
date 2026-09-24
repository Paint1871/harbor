//! Minimal stdio MCP server. Tokens stay in this process, never on the wire.

use std::io::{self, BufRead, Read, Write};

use serde_json::{Value, json};

use crate::github;

/// Same name the ACP `SpawnSpec` puts on the sidecar. Session only; never a token.
pub const SESSION_ENV: &str = "HARBOR_PLUGIN_SESSION";
/// Comma-separated plugin ids the host granted for this session's agent.
pub const GRANTS_ENV: &str = "HARBOR_PLUGIN_GRANTS";
/// Where the file-fallback credential store lives; the OS keyring is tried first.
pub const KEYRING_ENV: &str = "HARBOR_KEYRING_DIR";
/// Agent the session belongs to; folder threads pass none, and without one the
/// sidecar cannot raise approval requests because grants are per-agent.
pub const AGENT_ENV: &str = "HARBOR_PLUGIN_AGENT";
/// Harbor's SQLite path, so the sidecar can re-check grants live and record
/// approval requests. A path, never database contents.
pub const DB_ENV: &str = "HARBOR_DB";

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

/// Which provider a `github_*`-style tool belongs to.
fn provider_for_tool(tool: &str) -> Option<&'static str> {
    if tool.starts_with("github_") {
        Some("github")
    } else {
        None
    }
}

/// The sidecar's access to credentials and the network. The desktop binary
/// implements this with the OS keyring and its HTTP client; tests fake both.
pub trait ToolBackend {
    /// Granted for this session's agent AND a stored credential resolves.
    /// A granted-but-unauthenticated plugin still advertises nothing.
    fn plugin_available(&mut self, plugin: &str) -> bool;
    /// Runs one tool. Returns the text content for the MCP result.
    fn call_tool(&mut self, plugin: &str, tool: &str, args: &Value) -> Result<String, String>;
}

fn protocol_version(request: &Value) -> &str {
    request
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
        .filter(|version| !version.is_empty())
        .unwrap_or("2024-11-05")
}

fn tools_list(backend: &mut dyn ToolBackend) -> Value {
    let mut tools = Vec::new();
    if backend.plugin_available("github")
        && let Value::Array(specs) = github::tool_specs()
    {
        tools.extend(specs);
    }
    json!({ "tools": tools })
}

fn tools_call(backend: &mut dyn ToolBackend, params: Option<&Value>) -> Value {
    let Some(name) = params
        .and_then(|params| params.get("name"))
        .and_then(Value::as_str)
    else {
        return json!({
            "content": [{ "type": "text", "text": "tools/call needs a tool name" }],
            "isError": true
        });
    };
    let Some(plugin) = provider_for_tool(name) else {
        return json!({
            "content": [{ "type": "text", "text": format!("unknown tool: {name}") }],
            "isError": true
        });
    };
    let args = params
        .and_then(|params| params.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    match backend.call_tool(plugin, name, &args) {
        Ok(text) => json!({
            "content": [{ "type": "text", "text": text }],
            "isError": false
        }),
        Err(error) => json!({
            "content": [{ "type": "text", "text": error }],
            "isError": true
        }),
    }
}

/// Answer one JSON-RPC message. Notifications (no `id`) return `None`.
pub fn handle_message(backend: &mut dyn ToolBackend, message: &Value) -> Option<Value> {
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
            "result": tools_list(backend)
        })),
        "tools/call" => Some(json!({
            "jsonrpc": "2.0",
            "id": id?,
            "result": tools_call(backend, message.get("params"))
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

/// One JSON-RPC message is a few kilobytes of command; a megabyte is the
/// generous bound. A peer streaming a single endless line must not grow this
/// process without limit.
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// Consume the remainder of an over-long line so the next read starts on a
/// real message boundary again. `fill_buf`/`consume` scan without reading
/// past the newline — the leftover is just as untrusted as the part seen.
fn drain_line(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let (consumed, done) = {
            let buf = reader.fill_buf()?;
            if buf.is_empty() {
                return Ok(());
            }
            match buf.iter().position(|b| *b == b'\n') {
                Some(pos) => (pos + 1, true),
                None => (buf.len(), false),
            }
        };
        reader.consume(consumed);
        if done {
            return Ok(());
        }
    }
}

/// Newline-delimited JSON-RPC on stdio. Never echoes the request; never prints secrets.
pub fn serve<R, W>(mut reader: R, mut writer: W, backend: &mut dyn ToolBackend) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .by_ref()
            .take(MAX_MESSAGE_BYTES as u64 + 1)
            .read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        if line.len() > MAX_MESSAGE_BYTES {
            if !line.ends_with('\n') {
                drain_line(&mut reader)?;
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(response) = handle_message(backend, &message) else {
            continue;
        };
        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeBackend {
        available: bool,
    }

    impl ToolBackend for FakeBackend {
        fn plugin_available(&mut self, plugin: &str) -> bool {
            self.available && plugin == "github"
        }
        fn call_tool(&mut self, _plugin: &str, tool: &str, args: &Value) -> Result<String, String> {
            if tool == "github_fail" {
                return Err("boom".into());
            }
            Ok(format!("args:{}", args["owner"].as_str().unwrap_or("none")))
        }
    }

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
    fn handshake_and_empty_tools_when_nothing_is_available() {
        let mut backend = FakeBackend { available: false };
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"engine","version":"0"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            "\n"
        );
        let mut out = Vec::new();
        serve(input.as_bytes(), &mut out, &mut backend).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!looks_tokenish(&text));
        let mut lines = text.lines().filter(|line| !line.is_empty());
        let init: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(init["id"], 1);
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(init["result"]["serverInfo"]["name"], "harbor-plugins");
        let tools: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(tools["id"], 2);
        assert_eq!(tools["result"]["tools"], json!([]));
        assert!(lines.next().is_none(), "initialized is a notification");
    }

    #[test]
    fn available_plugin_lists_and_calls_real_tools() {
        let mut backend = FakeBackend { available: true };
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"github_repository","arguments":{"owner":"o","repo":"r"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"not_a_tool","arguments":{}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"github_fail","arguments":{}}}"#,
            "\n"
        );
        let mut out = Vec::new();
        serve(input.as_bytes(), &mut out, &mut backend).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!looks_tokenish(&text));
        let mut lines = text.lines().filter(|line| !line.is_empty());

        let list: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        let tools = list["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert!(tools.iter().all(|tool| {
            tool["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("github_"))
        }));

        let call: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(call["result"]["isError"], false);
        assert_eq!(call["result"]["content"][0]["text"], "args:o");

        let unknown: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(unknown["result"]["isError"], true);
        assert!(
            unknown["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("unknown tool")
        );

        let failed: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(failed["result"]["isError"], true);
        assert_eq!(failed["result"]["content"][0]["text"], "boom");
        assert!(lines.next().is_none());
    }

    #[test]
    fn an_overlong_line_is_dropped_and_framing_recovers() {
        let mut backend = FakeBackend { available: true };
        // A peer streaming one endless line must not grow this process; the
        // line is dropped and the next real message still parses.
        let mut input = vec![b'x'; MAX_MESSAGE_BYTES + 4096];
        input.push(b'\n');
        input.extend_from_slice(
            br#"{"jsonrpc":"2.0","id":9,"method":"tools/list","params":{}}
"#,
        );
        let mut out = Vec::new();
        serve(&input[..], &mut out, &mut backend).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1, "only the real message answered");
        let listed: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(listed["id"], 9);
    }
}
