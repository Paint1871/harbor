//! GitHub App + Device Flow. No client secret in the binary.

use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
/// Forks set this or `HARBOR_GITHUB_CLIENT_ID` to their GitHub App client id.
pub const CLIENT_ID: &str = "";
pub const CLIENT_ID_ENV: &str = "HARBOR_GITHUB_CLIENT_ID";
pub const APP_PERMISSIONS: &[(&str, &str)] = &[
    ("metadata", "read"),
    ("contents", "write"),
    ("issues", "write"),
    ("pull_requests", "write"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenPoll {
    Pending,
    SlowDown,
    Denied,
    Issued { token: String },
}

pub fn configured_client_id() -> Option<String> {
    std::env::var(CLIENT_ID_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            let baked = CLIENT_ID.trim();
            if baked.is_empty() {
                None
            } else {
                Some(baked.to_string())
            }
        })
}

fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", ch as u32)),
        }
    }
    out
}

pub fn device_request_body(client_id: &str) -> String {
    format!("client_id={}", form_encode(client_id))
}

pub fn token_poll_body(client_id: &str, device_code: &str) -> String {
    format!(
        "client_id={}&device_code={}&grant_type=urn:ietf:params:oauth:grant-type:device_code",
        form_encode(client_id),
        form_encode(device_code)
    )
}

pub fn refresh_body(client_id: &str, refresh_token: &str) -> String {
    format!(
        "client_id={}&refresh_token={}&grant_type=refresh_token",
        form_encode(client_id),
        form_encode(refresh_token)
    )
}

pub fn contains_client_secret(body: &str) -> bool {
    body.contains("client_secret")
}

pub fn run_device_flow<H, S>(
    client_id: &str,
    mut post: H,
    mut on_start: S,
) -> Result<String, String>
where
    H: FnMut(&str, &str) -> Result<String, String>,
    S: FnMut(&DeviceStart),
{
    if client_id.trim().is_empty() {
        return Err(
            "Set HARBOR_GITHUB_CLIENT_ID to your GitHub App client id (Device Flow enabled)".into(),
        );
    }
    let start_body = device_request_body(client_id);
    if contains_client_secret(&start_body) {
        return Err("client_secret must never be sent".into());
    }
    let start = parse_device_start(&post(DEVICE_CODE_URL, &start_body)?)?;
    on_start(&start);
    let deadline = Instant::now() + Duration::from_secs(start.expires_in.max(1));
    let mut wait = Duration::from_secs(start.interval);
    loop {
        if Instant::now() > deadline {
            return Err("device flow expired".into());
        }
        if !wait.is_zero() {
            thread::sleep(wait);
        }
        let poll_body = token_poll_body(client_id, &start.device_code);
        if contains_client_secret(&poll_body) {
            return Err("client_secret must never be sent".into());
        }
        match parse_token_poll(&post(TOKEN_URL, &poll_body)?)? {
            TokenPoll::Pending => wait = Duration::from_secs(start.interval.max(1)),
            TokenPoll::SlowDown => wait = Duration::from_secs(start.interval.max(1) + 5),
            TokenPoll::Denied => return Err("access denied".into()),
            TokenPoll::Issued { token } => return Ok(token),
        }
    }
}

pub fn parse_device_start(json: &str) -> Result<DeviceStart, String> {
    serde_json::from_str(json).map_err(|error| error.to_string())
}

pub fn parse_token_poll(json: &str) -> Result<TokenPoll, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if let Some(token) = value.get("access_token").and_then(|v| v.as_str()) {
        if !token.starts_with("ghu_") && !token.starts_with("gho_") {
            return Err("unexpected token prefix".into());
        }
        return Ok(TokenPoll::Issued {
            token: token.to_string(),
        });
    }
    Ok(match value.get("error").and_then(|v| v.as_str()) {
        Some("authorization_pending") => TokenPoll::Pending,
        Some("slow_down") => TokenPoll::SlowDown,
        Some("access_denied") | Some("expired_token") => TokenPoll::Denied,
        _ => TokenPoll::Denied,
    })
}

pub fn revoke_means_delete_keyring_only() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_and_token_json_without_a_secret() {
        let start = parse_device_start(
            r#"{"device_code":"dev","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","interval":5,"expires_in":900}"#,
        )
        .unwrap();
        assert_eq!(start.user_code, "ABCD-EFGH");
        assert!(device_request_body("client").contains("client_id=client"));
        assert!(!contains_client_secret(&device_request_body("client")));
        assert!(!device_request_body("client").contains("scope="));
        assert!(!contains_client_secret(&token_poll_body("client", "dev")));
        assert!(!contains_client_secret(&refresh_body("client", "ref")));
        assert!(APP_PERMISSIONS.iter().any(|(name, _)| *name == "contents"));
        assert!(matches!(
            parse_token_poll(r#"{"error":"authorization_pending"}"#).unwrap(),
            TokenPoll::Pending
        ));
        let issued = parse_token_poll(r#"{"access_token":"ghu_exampletoken"}"#).unwrap();
        assert!(matches!(issued, TokenPoll::Issued { .. }));
    }

    #[test]
    fn device_flow_polls_until_ghu_token_without_a_secret() {
        let mut urls = Vec::new();
        let token = run_device_flow(
            "Iv1.example",
            |url, body| {
                assert!(!contains_client_secret(body));
                urls.push(url.to_string());
                if url == DEVICE_CODE_URL {
                    Ok(r#"{"device_code":"dev","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","interval":0,"expires_in":9}"#.into())
                } else {
                    Ok(r#"{"access_token":"ghu_exampletoken"}"#.into())
                }
            },
            |start| assert_eq!(start.user_code, "ABCD-EFGH"),
        )
        .unwrap();
        assert_eq!(token, "ghu_exampletoken");
        assert_eq!(urls[0], DEVICE_CODE_URL);
        assert_eq!(urls[1], TOKEN_URL);
    }

    #[test]
    fn empty_client_id_is_refused() {
        let err = run_device_flow("", |_, _| Ok("{}".into()), |_| {}).unwrap_err();
        assert!(err.contains("HARBOR_GITHUB_CLIENT_ID"));
    }
}
