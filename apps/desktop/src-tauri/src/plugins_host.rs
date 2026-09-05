use harbor_core::SqlitePool;
use harbor_plugins::github::{self, DeviceStart};
use serde_json::json;
use tauri::{AppHandle, Emitter};

fn keyring_dir() -> std::path::PathBuf {
    crate::application_data_root().join("keyring")
}

fn post_form(url: &str, body: &str) -> Result<String, String> {
    reqwest::blocking::Client::new()
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .and_then(|response| response.text())
        .map_err(|error| error.to_string())
}

fn start_device(client_id: &str) -> Result<DeviceStart, String> {
    let body = github::device_request_body(client_id);
    if github::contains_client_secret(&body) {
        return Err("client_secret must never be sent".into());
    }
    github::parse_device_start(&post_form(github::DEVICE_CODE_URL, &body)?)
}

fn poll_existing(client_id: &str, start: &DeviceStart) -> Result<String, String> {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(start.expires_in.max(1));
    let mut wait = std::time::Duration::from_secs(start.interval);
    loop {
        if std::time::Instant::now() > deadline {
            return Err("device flow expired".into());
        }
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        let body = github::token_poll_body(client_id, &start.device_code);
        if github::contains_client_secret(&body) {
            return Err("client_secret must never be sent".into());
        }
        match github::parse_token_poll(&post_form(github::TOKEN_URL, &body)?)? {
            github::TokenPoll::Pending => {
                wait = std::time::Duration::from_secs(start.interval.max(1));
            }
            github::TokenPoll::SlowDown => {
                wait = std::time::Duration::from_secs(start.interval.max(1) + 5);
            }
            github::TokenPoll::Denied => return Err("access denied".into()),
            github::TokenPoll::Issued { token } => return Ok(token),
        }
    }
}

pub async fn connect(app: AppHandle, pool: SqlitePool, id: String) -> Result<(), String> {
    if id != "github" {
        return Err(format!("{id} is not a Harbor 0.1.0 plugin"));
    }
    let client_id = github::configured_client_id().ok_or_else(|| {
        "Set HARBOR_GITHUB_CLIENT_ID to your GitHub App client id (Device Flow enabled). Harbor never ships a client secret.".to_string()
    })?;
    let start = tauri::async_runtime::spawn_blocking({
        let client_id = client_id.clone();
        move || start_device(&client_id)
    })
    .await
    .map_err(|error| error.to_string())??;
    let _ = app.emit(
        "plugin_device",
        json!({
            "userCode": start.user_code,
            "verificationUri": start.verification_uri
        }),
    );
    tauri::async_runtime::spawn(async move {
        let token =
            tauri::async_runtime::spawn_blocking(move || poll_existing(&client_id, &start)).await;
        match token {
            Ok(Ok(token)) => {
                if let Err(error) = harbor_plugins::keyring::store(&keyring_dir(), "github", &token)
                {
                    let _ = app.emit("plugin_device", json!({ "error": error.to_string() }));
                    return;
                }
                let _ = harbor_core::commands::plugin_mark_connected(
                    &pool,
                    "github",
                    "GitHub",
                    Some("GitHub"),
                )
                .await;
            }
            Ok(Err(error)) => {
                let _ = app.emit("plugin_device", json!({ "error": error }));
            }
            Err(error) => {
                let _ = app.emit("plugin_device", json!({ "error": error.to_string() }));
            }
        }
    });
    Ok(())
}

pub async fn disconnect(pool: &SqlitePool, id: String) -> Result<(), String> {
    harbor_plugins::keyring::delete(&keyring_dir(), &id).map_err(|error| error.to_string())?;
    harbor_core::commands::plugin_mark_disconnected(pool, &id)
        .await
        .map_err(|error| error.to_string())
}
