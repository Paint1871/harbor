use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

use crate::AcpError;
use crate::spawn::SpawnSpec;

pub struct AcpConn {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    pub notifications: Vec<Value>,
}

impl AcpConn {
    pub fn spawn(spec: &SpawnSpec) -> Result<Self, AcpError> {
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            notifications: Vec::new(),
        })
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, AcpError> {
        let id = self.next_id;
        self.next_id += 1;
        write_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        )?;
        loop {
            let incoming = read_message(&mut self.stdout)?;
            if incoming.get("id") == Some(&json!(id)) {
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
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "outcome": "cancelled" }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "Method not found" }
                })
            };
        write_message(&mut self.stdin, &body)
    }
}

impl Drop for AcpConn {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub fn write_message<W: Write>(writer: &mut W, body: &Value) -> Result<(), AcpError> {
    let bytes = serde_json::to_vec(body)?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

pub fn read_message<R: BufRead + Read>(reader: &mut R) -> Result<Value, AcpError> {
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(AcpError::Protocol("eof"));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        headers.push_str(&line);
    }
    let length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")?
                .trim()
                .parse::<usize>()
                .ok()
        })
        .ok_or(AcpError::Protocol("content-length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn content_length_roundtrip() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"ok": true, "n": 1})).unwrap();
        let header = String::from_utf8_lossy(&buf);
        assert!(header.starts_with("Content-Length: "));
        assert!(header.contains("\r\n\r\n"));
        let value = read_message(&mut Cursor::new(buf)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["n"], 1);
    }
}
