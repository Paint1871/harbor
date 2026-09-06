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

async fn github_client_id(pool: &SqlitePool) -> Result<String, String> {
    let setting = harbor_core::settings::get(pool, github::CLIENT_ID_SETTING)
        .await
        .ok()
        .flatten();
    github::resolve_client_id(setting.as_ref().and_then(|value| value.as_str()))
        .ok_or_else(github::missing_client_id_error)
}

pub async fn connect(app: AppHandle, pool: SqlitePool, id: String) -> Result<(), String> {
    if id != "github" {
        if harbor_core::plugins::supports_manual_token(&id) {
            return Err("This connection uses a local credential. Use its setup form.".into());
        }
        return Err(format!("unsupported Harbor connection: {id}"));
    }
    let client_id = github_client_id(&pool).await?;
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
                let _ = app.emit(
                    "plugin_device",
                    json!({ "connected": true, "id": "github" }),
                );
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

/// Store a user-supplied integration credential in the OS keyring. The value
/// never touches SQLite, the renderer logs, or an engine environment.
pub async fn configure(
    pool: &SqlitePool,
    id: String,
    credential: String,
    account_label: Option<String>,
) -> Result<(), String> {
    let definition = harbor_core::plugins::definition(&id)
        .ok_or_else(|| format!("unsupported Harbor connection: {id}"))?;
    if definition.auth_kind != "token" {
        return Err("This connection uses its browser sign-in flow.".into());
    }
    let credential = credential.trim().to_string();
    if credential.is_empty() {
        return Err("Enter a credential before saving the connection.".into());
    }
    if credential.len() > 4096 {
        return Err("That credential is too long to store.".into());
    }
    let account_label = account_label
        .unwrap_or_default()
        .trim()
        .chars()
        .take(120)
        .collect::<String>();
    let keyring_path = keyring_dir();
    let keyring_id = id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        harbor_plugins::keyring::store(&keyring_path, &keyring_id, &credential)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;

    harbor_core::commands::plugin_mark_connected(
        pool,
        &id,
        definition.display_name,
        (!account_label.is_empty()).then_some(account_label.as_str()),
    )
    .await
    .map_err(|error| error.to_string())
}

pub async fn disconnect(pool: &SqlitePool, id: String) -> Result<(), String> {
    harbor_plugins::keyring::delete(&keyring_dir(), &id).map_err(|error| error.to_string())?;
    harbor_core::commands::plugin_mark_disconnected(pool, &id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn set_agent_grant(
    pool: &SqlitePool,
    agent_id: String,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    harbor_core::commands::plugin_set_agent_grant(pool, agent_id, plugin_id, enabled)
        .await
        .map_err(|error| error.to_string())
}

pub async fn resolve_approval(pool: &SqlitePool, id: String, allow: bool) -> Result<(), String> {
    harbor_core::commands::plugin_resolve_approval(pool, id, allow)
        .await
        .map_err(|error| error.to_string())
}
