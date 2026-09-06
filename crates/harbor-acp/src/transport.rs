use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::AcpError;
use crate::permissions::permission_outcome;
use crate::spawn::SpawnSpec;

/// Host callback for ACP v1 `session/request_permission`. Returns the inner
/// outcome object (`selected` + `optionId`, or `cancelled`).
pub type PermissionHook = Arc<dyn Fn(Value) -> Result<Value, AcpError> + Send + Sync>;

pub struct AcpConn {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
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
        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: BufReader::new(stdout),
            next_id: 1,
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
        self.write_rpc(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, AcpError> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_rpc(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        loop {
            let incoming = read_message(&mut self.stdout)?;
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
