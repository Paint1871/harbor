use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use harbor_paths::{ShellKind, quote_for_shell, refuse_if_expandable};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("executable path must be absolute")]
    Relative,
    #[error("executable is not on the host allowlist")]
    Denied,
    #[error(transparent)]
    Path(#[from] harbor_paths::PathError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl PartialEq for PtyError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Relative, Self::Relative) | (Self::Denied, Self::Denied)
        )
    }
}

#[derive(Debug, Clone)]
pub struct PtySpawn {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub fn prepare_spawn(
    program: &Path,
    cwd: &Path,
    allowlisted: &[PathBuf],
) -> Result<PtySpawn, PtyError> {
    if !program.is_absolute() {
        return Err(PtyError::Relative);
    }
    if !allowlisted.iter().any(|granted| granted == program) {
        return Err(PtyError::Denied);
    }
    Ok(PtySpawn {
        program: program.to_path_buf(),
        args: Vec::new(),
        cwd: cwd.to_path_buf(),
    })
}

pub fn typed_path(raw: &str, unix: bool) -> Result<String, PtyError> {
    refuse_if_expandable(raw)?;
    Ok(quote_for_shell(
        Path::new(raw),
        if unix {
            ShellKind::Unix
        } else {
            ShellKind::PowerShell
        },
    )?)
}

pub struct LivePty {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl LivePty {
    pub fn spawn(
        program: &Path,
        cwd: &Path,
        cols: u16,
        rows: u16,
        allowlisted: &[PathBuf],
    ) -> Result<(Self, Receiver<Vec<u8>>), PtyError> {
        let request = prepare_spawn(program, cwd, allowlisted)?;
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io::Error::other)?;
        let mut cmd = CommandBuilder::new(request.program);
        cmd.cwd(request.cwd);
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0_u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok((
            Self {
                writer: Mutex::new(writer),
                master: Mutex::new(pair.master),
                child: Mutex::new(child),
            },
            rx,
        ))
    }

    pub fn write(&self, bytes: &[u8]) -> io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| io::Error::other("pty writer"))?;
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        self.master
            .lock()
            .map_err(|_| io::Error::other("pty master"))?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io::Error::other)
    }

    pub fn kill(&self) -> io::Result<()> {
        self.child
            .lock()
            .map_err(|_| io::Error::other("pty child"))?
            .kill()
    }
}

pub fn recv_deadline(rx: &Receiver<Vec<u8>>, needle: &str, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    let mut acc = String::new();
    while std::time::Instant::now() < deadline {
        let wait = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(wait.min(Duration::from_millis(50))) {
            Ok(chunk) => {
                acc.push_str(&String::from_utf8_lossy(&chunk));
                if acc.contains(needle) {
                    return true;
                }
            }
            Err(_) => continue,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_relative_and_unlisted_programs() {
        let allow = vec![PathBuf::from("/bin/zsh")];
        assert_eq!(
            prepare_spawn(Path::new("zsh"), Path::new("/tmp"), &allow).unwrap_err(),
            PtyError::Relative
        );
        assert_eq!(
            prepare_spawn(Path::new("/bin/bash"), Path::new("/tmp"), &allow).unwrap_err(),
            PtyError::Denied
        );
        assert!(prepare_spawn(Path::new("/bin/zsh"), Path::new("/tmp"), &allow).is_ok());
    }

    #[test]
    fn quoted_paths_refuse_expansion() {
        assert!(typed_path("$HOME/x", true).is_err());
        assert_eq!(typed_path("/tmp/file", true).unwrap(), "'/tmp/file'");
    }

    #[cfg(unix)]
    #[test]
    fn spawns_allowlisted_shell_and_echoes() {
        let sh = PathBuf::from("/bin/sh");
        let (pty, rx) = LivePty::spawn(&sh, Path::new("/"), 40, 12, &[sh.clone()]).unwrap();
        pty.write(b"printf 'harbor-pty-ok\\n'\n").unwrap();
        assert!(
            recv_deadline(&rx, "harbor-pty-ok", Duration::from_secs(3)),
            "PTY never printed harbor-pty-ok"
        );
        let _ = pty.kill();
    }
}
