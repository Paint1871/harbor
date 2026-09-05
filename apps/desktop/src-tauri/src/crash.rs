use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn prepare_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Never log the panic payload: it can contain prompts, environment values, or tokens.
pub fn install(data_root: &Path) {
    let log_dir = data_root.join("logs");
    std::panic::set_hook(Box::new(move |info| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "unknown location".into());
        let summary = format!(
            "{timestamp} Harbor {} panic at {location}; payload omitted",
            crate::version()
        );
        if prepare_directory(&log_dir).is_ok() {
            let mut options = OpenOptions::new();
            options.create(true).append(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            if let Ok(mut file) = options.open(log_dir.join("crash.log")) {
                let _ = writeln!(file, "{summary}");
                let _ = file.sync_data();
            }
        }
        #[cfg(target_os = "macos")]
        oslog::OsLog::new("app.harbor.desktop", "crash").fault(&summary);
        #[cfg(not(target_os = "macos"))]
        eprintln!("{summary}");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_child() {
        if let Some(root) = std::env::var_os("HARBOR_TEST_CRASH_ROOT") {
            install(Path::new(&root));
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                panic!("test-secret-that-must-not-be-logged");
            }));
            // Leave through exit so the test harness cannot print the payload.
            std::process::exit(1);
        }
    }

    #[test]
    fn installed_hook_records_crash_without_secret_payload() {
        let root = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash::tests::panic_child", "--nocapture"])
            .env("HARBOR_TEST_CRASH_ROOT", root.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        let path = root.path().join("logs/crash.log");
        let log = fs::read_to_string(&path).unwrap();
        assert!(log.contains("panic at"));
        assert!(log.contains("payload omitted"));
        assert!(!log.contains("test-secret"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("test-secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
