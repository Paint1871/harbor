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

/// Settings key for a user-supplied GitHub App client id. Never a client secret.
pub const CLIENT_ID_SETTING: &str = "github_client_id";

pub fn resolve_client_id(settings_value: Option<&str>) -> Option<String> {
    configured_client_id().or_else(|| {
        settings_value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub fn missing_client_id_error() -> String {
    "Set a GitHub App client id under Settings → General (github_client_id) or HARBOR_GITHUB_CLIENT_ID, with Device Flow enabled on the app. Harbor never ships a client secret.".into()
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

pub fn contains_client_secret(body: &str) -> bool {
    body.contains("client_secret")
}

/// Poll the token endpoint until the device grant is issued, denied, or
/// expires. `expires_in` and `interval` come from the server, so they are
/// clamped to sane bounds instead of trusted: a garbage `expires_in` cannot
/// overflow the deadline and a garbage `interval` cannot stall the loop.
pub fn poll_device_token<H>(
    client_id: &str,
    start: &DeviceStart,
    mut post: H,
) -> Result<String, String>
where
    H: FnMut(&str, &str) -> Result<String, String>,
{
    let deadline = Instant::now() + Duration::from_secs(start.expires_in.clamp(1, 3600));
    let mut wait = Duration::from_secs(start.interval.min(120));
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
            TokenPoll::Pending => {
                wait = Duration::from_secs(start.interval.clamp(1, 120));
            }
            TokenPoll::SlowDown => {
                wait = Duration::from_secs(start.interval.saturating_add(5).clamp(1, 300));
            }
            TokenPoll::Denied => return Err("access denied".into()),
            TokenPoll::Issued { token } => return Ok(token),
        }
    }
}

/// The verification link leaves the app through the system browser, so it
/// must be a real GitHub https URL — a compromised or buggy response must
/// never send the builder's browser somewhere else carrying the user code.
fn verification_uri_allowed(uri: &str) -> bool {
    let Some(rest) = uri.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    host == "github.com" || host.ends_with(".github.com")
}

pub fn parse_device_start(json: &str) -> Result<DeviceStart, String> {
    let start: DeviceStart = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if !verification_uri_allowed(&start.verification_uri) {
        return Err("device flow returned an unexpected verification URL".into());
    }
    Ok(start)
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

// --- MCP tool surface ------------------------------------------------------
//
// Read-only by design: write tools land with the approval flow, not before.
// The sidecar maps each tool to one GET against the public REST API and
// projects the response down to the fields an engine can actually use.

pub const API_BASE: &str = "https://api.github.com";
const MAX_LIMIT: u64 = 50;
/// File bodies are capped so one tool call cannot flood the engine context.
const MAX_FILE_CHARS: usize = 50_000;

/// One REST call the host performs on the tool's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRequest {
    pub url: String,
    /// `application/vnd.github.raw` returns the file body instead of JSON.
    pub raw: bool,
}

fn repo_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn file_path_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | '~'))
}

fn arg_string<'a>(args: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing argument: {name}"))
}

fn arg_limit(args: &serde_json::Value) -> u64 {
    args.get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(10)
        .clamp(1, MAX_LIMIT)
}

fn arg_state(args: &serde_json::Value) -> &str {
    match args.get("state").and_then(|value| value.as_str()) {
        Some(state @ ("closed" | "all")) => state,
        _ => "open",
    }
}

fn repo_args(args: &serde_json::Value) -> Result<(&str, &str), String> {
    let owner = arg_string(args, "owner")?;
    let repo = arg_string(args, "repo")?;
    if !repo_segment(owner) || !repo_segment(repo) {
        return Err("owner and repo may only contain letters, digits, - _ .".into());
    }
    Ok((owner, repo))
}

/// The MCP `tools/list` payload for the GitHub provider.
pub fn tool_specs() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "github_me",
            "description": "Show which GitHub account the stored credential belongs to.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "github_repository",
            "description": "Read a repository's summary: description, language, stars, default branch.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "repo": { "type": "string" }
                },
                "required": ["owner", "repo"],
                "additionalProperties": false
            }
        },
        {
            "name": "github_issues",
            "description": "List issues in a repository (pull requests are excluded).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "repo": { "type": "string" },
                    "state": { "type": "string", "enum": ["open", "closed", "all"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT }
                },
                "required": ["owner", "repo"],
                "additionalProperties": false
            }
        },
        {
            "name": "github_pull_requests",
            "description": "List pull requests in a repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "repo": { "type": "string" },
                    "state": { "type": "string", "enum": ["open", "closed", "all"] },
                    "limit": { "type": "integer", "minimum": 1, "maximum": MAX_LIMIT }
                },
                "required": ["owner", "repo"],
                "additionalProperties": false
            }
        },
        {
            "name": "github_read_file",
            "description": "Read one file's contents from a repository path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string" },
                    "repo": { "type": "string" },
                    "path": { "type": "string" },
                    "ref": { "type": "string" }
                },
                "required": ["owner", "repo", "path"],
                "additionalProperties": false
            }
        }
    ])
}

/// Map a `tools/call` to the REST request the host should perform.
pub fn tool_request(tool: &str, args: &serde_json::Value) -> Result<ApiRequest, String> {
    match tool {
        "github_me" => Ok(ApiRequest {
            url: format!("{API_BASE}/user"),
            raw: false,
        }),
        "github_repository" => {
            let (owner, repo) = repo_args(args)?;
            Ok(ApiRequest {
                url: format!("{API_BASE}/repos/{owner}/{repo}"),
                raw: false,
            })
        }
        "github_issues" | "github_pull_requests" => {
            let (owner, repo) = repo_args(args)?;
            let kind = if tool == "github_issues" {
                "issues"
            } else {
                "pulls"
            };
            Ok(ApiRequest {
                url: format!(
                    "{API_BASE}/repos/{owner}/{repo}/{kind}?state={}&per_page={}",
                    arg_state(args),
                    arg_limit(args)
                ),
                raw: false,
            })
        }
        "github_read_file" => {
            let (owner, repo) = repo_args(args)?;
            let path = arg_string(args, "path")?;
            if !file_path_segment(path) {
                return Err("path may only contain letters, digits, - _ . / ~".into());
            }
            let mut url = format!("{API_BASE}/repos/{owner}/{repo}/contents/{path}");
            if let Ok(git_ref) = arg_string(args, "ref")
                && git_ref
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/'))
            {
                url.push_str(&format!("?ref={git_ref}"));
            }
            Ok(ApiRequest { url, raw: true })
        }
        _ => Err(format!("unknown GitHub tool: {tool}")),
    }
}

fn pick(value: &serde_json::Value, name: &str) -> serde_json::Value {
    value.get(name).cloned().unwrap_or(serde_json::Value::Null)
}

fn issue_row(item: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "number": pick(item, "number"),
        "title": pick(item, "title"),
        "state": pick(item, "state"),
        "author": item.pointer("/user/login").cloned().unwrap_or(serde_json::Value::Null),
        "labels": item
            .get("labels")
            .and_then(|labels| labels.as_array())
            .map(|labels| labels.iter().filter_map(|label| label.get("name")).collect::<Vec<_>>())
            .unwrap_or_default(),
        "comments": pick(item, "comments"),
        "created_at": pick(item, "created_at"),
        "url": pick(item, "html_url"),
    })
}

/// Project the REST response down to the fields an engine can use.
pub fn shape_tool_result(tool: &str, body: &str) -> Result<String, String> {
    if tool == "github_read_file" {
        return Ok(body.chars().take(MAX_FILE_CHARS).collect());
    }
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "GitHub returned invalid JSON".to_string())?;
    let shaped = match tool {
        "github_me" => serde_json::json!({
            "login": pick(&value, "login"),
            "name": pick(&value, "name"),
            "id": pick(&value, "id"),
            "url": pick(&value, "html_url"),
        }),
        "github_repository" => serde_json::json!({
            "full_name": pick(&value, "full_name"),
            "description": pick(&value, "description"),
            "private": pick(&value, "private"),
            "language": pick(&value, "language"),
            "stars": pick(&value, "stargazers_count"),
            "forks": pick(&value, "forks_count"),
            "open_issues": pick(&value, "open_issues_count"),
            "default_branch": pick(&value, "default_branch"),
            "url": pick(&value, "html_url"),
        }),
        "github_issues" | "github_pull_requests" => {
            let items = value.as_array().cloned().unwrap_or_default();
            let rows: Vec<_> = items
                .iter()
                .filter(|item| tool != "github_issues" || item.get("pull_request").is_none())
                .map(issue_row)
                .collect();
            serde_json::Value::Array(rows)
        }
        _ => return Err(format!("unknown GitHub tool: {tool}")),
    };
    serde_json::to_string_pretty(&shaped).map_err(|error| error.to_string())
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
        let start = parse_device_start(
            r#"{"device_code":"dev","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","interval":0,"expires_in":9}"#,
        )
        .unwrap();
        assert_eq!(start.user_code, "ABCD-EFGH");
        let token = poll_device_token("Iv1.example", &start, |url, body| {
            assert_eq!(url, TOKEN_URL);
            assert!(!contains_client_secret(body));
            Ok(r#"{"access_token":"ghu_exampletoken"}"#.into())
        })
        .unwrap();
        assert_eq!(token, "ghu_exampletoken");
    }

    #[test]
    fn a_denied_or_expired_grant_is_an_error_not_a_hang() {
        let start = parse_device_start(
            r#"{"device_code":"dev","user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","interval":0,"expires_in":9}"#,
        )
        .unwrap();
        let denied = poll_device_token("Iv1.example", &start, |_, _| {
            Ok(r#"{"error":"access_denied"}"#.into())
        });
        assert_eq!(denied, Err("access denied".into()));
    }

    #[test]
    fn settings_client_id_is_used_when_env_is_empty() {
        assert!(CLIENT_ID.is_empty());
        if configured_client_id().is_none() {
            assert_eq!(
                resolve_client_id(Some("Iv1.from-settings")),
                Some("Iv1.from-settings".into())
            );
            assert!(resolve_client_id(Some("")).is_none());
        }
        assert!(missing_client_id_error().contains("github_client_id"));
        assert!(missing_client_id_error().contains("never ships a client secret"));
    }
}
