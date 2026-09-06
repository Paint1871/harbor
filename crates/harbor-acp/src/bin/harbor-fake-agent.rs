//! In-process ACP v1 fixture. Caps come from HARBOR_FAKE_ACP_FIXTURE.

use std::io::{self, BufRead, BufReader, Write};

use serde_json::{Value, json};

fn main() -> io::Result<()> {
    let fixture = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("HARBOR_FAKE_ACP_FIXTURE").ok())
        .unwrap_or_else(|| "neither".into());
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = io::stdout();
    let mut line = String::new();
    let mut pending_prompt: Option<Value> = None;
    while stdin.read_line(&mut line)? > 0 {
        let message: Value = serde_json::from_str(&line)?;
        line.clear();
        if pending_prompt.is_some()
            && (message.get("result").is_some() || message.get("error").is_some())
        {
            let prompt_id = pending_prompt.take();
            notify(
                &mut stdout,
                json!({
                    "jsonrpc": "2.0",
                    "method": "session/update",
                    "params": { "sessionId": "sess-fixture", "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "ok" } } }
                }),
            )?;
            respond(&mut stdout, prompt_id, json!({ "stopReason": "end_turn" }))?;
            continue;
        }
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
            "session/prompt" if fixture == "permissions" => {
                pending_prompt = id;
                write_rpc(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": 1000,
                        "method": "session/request_permission",
                        "params": {
                            "sessionId": "sess-fixture",
                            "toolCall": { "title": "Run tests", "path": "/tmp/proj", "command": "cargo test" },
                            "options": [
                                { "optionId": "opt-allow", "kind": "allow_once", "name": "Allow" },
                                { "optionId": "opt-always", "kind": "allow_always", "name": "Allow for session" },
                                { "optionId": "opt-deny", "kind": "reject_once", "name": "Deny" }
                            ]
                        }
                    }),
                )?;
            }
            "session/prompt" => {
                notify(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": { "sessionId": "sess-fixture", "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "ok" } } }
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
    serde_json::to_writer(&mut *stdout, body)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}
