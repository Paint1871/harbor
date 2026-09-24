use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::AcpError;
use crate::permissions::permission_outcome;
use crate::spawn::SpawnSpec;

/// Host callback for ACP v1 `session/request_permission`. Returns the inner
/// outcome object (`selected` + `optionId`, or `cancelled`).
pub type PermissionHook = Arc<dyn Fn(Value) -> Result<Value, AcpError> + Send + Sync>;

/// Control calls — initialize, session open, config writes — must answer
/// quickly; a silent engine is a broken engine, not a slow one.
pub const CONTROL_IDLE: Duration = Duration::from_secs(60);
/// A turn may legitimately run for a long time, but a turn that produces no
/// message at all for this long is hung: streaming updates and permission
/// requests each reset the window.
pub const TURN_IDLE: Duration = Duration::from_secs(30 * 60);

pub struct AcpConn {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    inbound: mpsc::Receiver<Value>,
    next_id: i64,
    /// Set once the engine can no longer be trusted — a deadline trip or a
    /// closed stream means a later reply could pair with the wrong request.
    dead: bool,
    pub notifications: Vec<Value>,
    pub permission_hook: Option<PermissionHook>,
}

impl AcpConn {
    pub fn spawn(spec: &SpawnSpec) -> Result<Self, AcpError> {
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or(AcpError::Protocol("stdin"))?;
        let stdout = child.stdout.take().ok_or(AcpError::Protocol("stdout"))?;
        // A dedicated reader owns stdout so callers can bound their wait with
        // recv_timeout; when the process dies the channel closes.
        let (tx, inbound) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            while let Ok(message) = read_message(&mut stdout) {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            inbound,
            next_id: 1,
            dead: false,
            notifications: Vec::new(),
            permission_hook: None,
        })
    }

    pub fn stdin_handle(&self) -> Arc<Mutex<ChildStdin>> {
        self.stdin.clone()
    }

    fn write_rpc(&self, body: &Value) -> Result<(), AcpError> {
        let mut stdin = self
            .stdin
            .lock()
            .map_err(|_| AcpError::Protocol("stdin lock"))?;
        write_message(&mut *stdin, body)
    }

    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), AcpError> {
        if self.dead {
            return Err(AcpError::Protocol("engine process is gone"));
        }
        self.write_rpc(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, AcpError> {
        self.request_idle(method, params, CONTROL_IDLE)
    }

    /// A request bounded by silence: any inbound message — updates, permission
    /// requests, the answer — resets the window. `idle` is the longest stretch
    /// with no message at all before the engine is declared hung and killed.
    pub fn request_idle(
        &mut self,
        method: &str,
        params: Value,
        idle: Duration,
    ) -> Result<Value, AcpError> {
        if self.dead {
            return Err(AcpError::Protocol("engine process is gone"));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.write_rpc(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        loop {
            let incoming = match self.inbound.recv_timeout(idle) {
                Ok(incoming) => incoming,
                Err(RecvTimeoutError::Timeout) => {
                    self.dead = true;
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Err(AcpError::Protocol("engine timed out"));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.dead = true;
                    return Err(AcpError::Protocol("engine exited"));
                }
            };
            if incoming.get("id") == Some(&json!(id))
                && (incoming.get("result").is_some() || incoming.get("error").is_some())
            {
                if incoming.get("error").is_some() {
                    return Err(AcpError::Protocol("rpc error"));
                }
                return Ok(incoming.get("result").cloned().unwrap_or(Value::Null));
            }
            if incoming.get("method").is_some()
                && incoming.get("id").is_some()
                && incoming.get("result").is_none()
            {
                self.reply_to_agent(&incoming)?;
                continue;
            }
            self.notifications.push(incoming);
        }
    }

    fn reply_to_agent(&mut self, incoming: &Value) -> Result<(), AcpError> {
        let id = incoming.get("id").cloned().unwrap_or(Value::Null);
        let method = incoming.get("method").and_then(Value::as_str).unwrap_or("");
        let body =
            if method.ends_with("request_permission") || method.ends_with("requestPermission") {
                let params = incoming.get("params").cloned().unwrap_or(Value::Null);
                let inner = match &self.permission_hook {
                    Some(hook) => hook(params).unwrap_or_else(|_| permission_outcome(None, true)),
                    None => permission_outcome(None, true),
                };
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "outcome": inner }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "Method not found" }
                })
            };
        self.write_rpc(&body)
    }
}

impl Drop for AcpConn {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn write_message<W: Write>(writer: &mut W, body: &Value) -> Result<(), AcpError> {
    serde_json::to_writer(&mut *writer, body)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub fn read_message<R: BufRead + Read>(reader: &mut R) -> Result<Value, AcpError> {
    const MAX_FRAME: u64 = 16 * 1024 * 1024;
    let mut body = Vec::new();
    let count = reader.take(MAX_FRAME + 1).read_until(b'\n', &mut body)?;
    if count == 0 {
        return Err(AcpError::Protocol("eof"));
    }
    if count as u64 > MAX_FRAME {
        return Err(AcpError::Protocol("message too large"));
    }
    if body.last() != Some(&b'\n') {
        return Err(AcpError::Protocol("unterminated message"));
    }
    Ok(serde_json::from_slice(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_independent_newline_delimited_protocol_messages() {
        let mut input = Cursor::new(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{}}\n");
        assert_eq!(read_message(&mut input).unwrap()["id"], 1);
        assert_eq!(
            read_message(&mut input).unwrap()["method"],
            "session/update"
        );
        assert!(read_message(&mut input).is_err());
    }

    #[test]
    fn writes_one_json_line_and_escapes_message_newlines() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"text": "hello\nworld"})).unwrap();
        assert_eq!(buf, b"{\"text\":\"hello\\nworld\"}\n");
        assert!(read_message(&mut Cursor::new(b"Content-Length: 2\r\n\r\n{}")).is_err());
        assert!(read_message(&mut Cursor::new(b"{}")).is_err());
    }
}
