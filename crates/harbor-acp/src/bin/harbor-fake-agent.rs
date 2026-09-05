//! In-process ACP v1 fixture. Caps come from HARBOR_FAKE_ACP_FIXTURE.

use std::io::{self, BufReader, Write};

use harbor_acp::transport::{read_message, write_message};
use serde_json::{Value, json};

fn main() -> io::Result<()> {
    let fixture = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("HARBOR_FAKE_ACP_FIXTURE").ok())
        .unwrap_or_else(|| "neither".into());
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = io::stdout();
    while let Ok(message) = read_message(&mut stdin) {
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match method {
            "initialize" => {
                respond(&mut stdout, id, initialize_result(&fixture))?;
            }
            "session/new" | "session/resume" | "session/load" => {
                if fixture == "load_only" && method == "session/load" {
                    notify(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "method": "session/update",
                            "params": { "sessionUpdate": "replay", "messageId": "replay-1" }
                        }),
                    )?;
                }
                respond(&mut stdout, id, json!({ "sessionId": "sess-fixture" }))?;
            }
            "session/prompt" => {
                notify(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "ok" } }
                    }),
                )?;
                respond(&mut stdout, id, json!({ "stopReason": "end_turn" }))?;
            }
            "session/cancel" => {
                respond(&mut stdout, id, json!({}))?;
            }
            "session/request_permission" => {
                let _ = params;
                respond(&mut stdout, id, json!({ "outcome": "cancelled" }))?;
            }
            _ => {
                if let Some(id) = id {
                    respond(&mut stdout, Some(id), json!({}))?;
                }
            }
        }
    }
    Ok(())
}

fn initialize_result(fixture: &str) -> Value {
    match fixture {
        "resume_only" => json!({
            "protocolVersion": 1,
            "agentCapabilities": { "sessionCapabilities": { "resume": {} } }
        }),
        "load_only" => json!({
            "protocolVersion": 1,
            "agentCapabilities": { "loadSession": true }
        }),
        "add_dirs" => json!({
            "protocolVersion": 1,
            "agentCapabilities": { "sessionCapabilities": { "additionalDirectories": {} } }
        }),
        "config" => json!({
            "protocolVersion": 1,
            "agentCapabilities": {},
            "configOptions": [
                { "id": "model", "category": "model" },
                { "id": "mode", "category": "mode" }
            ]
        }),
        _ => json!({ "protocolVersion": 1, "agentCapabilities": {} }),
    }
}

fn respond(stdout: &mut impl Write, id: Option<Value>, result: Value) -> io::Result<()> {
    let Some(id) = id else { return Ok(()) };
    write_rpc(
        stdout,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

fn notify(stdout: &mut impl Write, body: Value) -> io::Result<()> {
    write_rpc(stdout, &body)
}

fn write_rpc(stdout: &mut impl Write, body: &Value) -> io::Result<()> {
    write_message(stdout, body).map_err(io::Error::other)
}
