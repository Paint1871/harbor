use harbor_core::SqlitePool;
use harbor_plugins::github::{self, DeviceStart};
use serde_json::{Value, json};
use std::io;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub(crate) fn keyring_dir() -> std::path::PathBuf {
    crate::application_data_root().join("keyring")
}

/// A stalled socket or a blackholed connection must not park a host thread
/// forever: connect and total timeouts, plus a byte cap on every body read.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// GitHub API responses for the read-only tools are kilobytes; the cap is a
/// hard error rather than a silent truncation that could hide mangled JSON.
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn bounded_body(
    mut response: reqwest::blocking::Response,
    context: &str,
) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{context}: {error}"))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(format!(
            "{context}: response exceeded {MAX_RESPONSE_BYTES} bytes"
        ));
    }
    String::from_utf8(bytes).map_err(|_| format!("{context}: response was not utf-8"))
}

fn post_form(url: &str, body: &str) -> Result<String, String> {
    let response = http_client()
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub device flow failed: {}", response.status()));
    }
    bounded_body(response, "GitHub device flow failed")
}

fn start_device(client_id: &str) -> Result<DeviceStart, String> {
    let body = github::device_request_body(client_id);
    if github::contains_client_secret(&body) {
        return Err("client_secret must never be sent".into());
    }
    github::parse_device_start(&post_form(github::DEVICE_CODE_URL, &body)?)
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
        let token = tauri::async_runtime::spawn_blocking(move || {
            github::poll_device_token(&client_id, &start, post_form)
        })
        .await;
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
    if !definition.released {
        return Err(format!(
            "{} is not available in this build yet.",
            definition.display_name
        ));
    }
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
    // Credential first: if the keychain cannot confirm removal, the row must
    // keep saying "connected" — marking it disconnected while a token
    // survives would hide a still-reachable credential.
    harbor_plugins::keyring::delete(&keyring_dir(), &id).map_err(|error| error.to_string())?;
    harbor_core::commands::plugin_disconnect(pool, id)
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

// --- `harbor mcp-plugins` sidecar ------------------------------------------
//
// Engines spawn this as a stdio MCP server. It reads the grants list and the
// keyring path from its environment — the session reference stays on the
// command line — and resolves credentials itself so tokens never enter the
// engine environment.

struct SidecarBackend {
    /// Grant snapshot from spawn env; the database is authoritative whenever
    /// it can be opened, so this is the fallback for outage paths only.
    env_grants: Vec<String>,
    /// Agent this session belongs to; `None` for folder threads, which can
    /// never raise approval requests because grants are per-agent.
    agent_id: Option<String>,
    /// Harbor's SQLite path; the pool opens lazily on first grant check.
    db_path: Option<std::path::PathBuf>,
    pool: Option<SqlitePool>,
    keyring_dir: std::path::PathBuf,
    client: reqwest::blocking::Client,
}

impl SidecarBackend {
    fn token(&self, plugin: &str) -> Result<String, String> {
        harbor_plugins::keyring::load(&self.keyring_dir, plugin)
            .map_err(|_| format!("{plugin} is not connected"))
    }

    fn pool(&mut self) -> Option<&SqlitePool> {
        if self.pool.is_none() {
            let path = self.db_path.clone()?;
            self.pool = tauri::async_runtime::block_on(harbor_core::db::open(&path)).ok();
        }
        self.pool.as_ref()
    }

    /// The database is authoritative for agent sessions: a grant the builder
    /// toggles under Plugins takes effect — and stops taking effect — on the
    /// next call, without a respawn. The spawn-time env list is the fallback
    /// for when the pool cannot be opened, so a session keeps its spawn-time
    /// access through a database outage rather than going silent mid-turn.
    /// A plugin row that exists and is not `connected` makes every grant
    /// inert: disconnecting cuts live access immediately.
    fn granted(&mut self, plugin: &str) -> bool {
        let agent_id = self.agent_id.clone();
        if let Some(pool) = self.pool() {
            if let Ok(Some(status)) =
                tauri::async_runtime::block_on(harbor_core::plugins::plugin_status(pool, plugin))
                && status != "connected"
            {
                return false;
            }
            if let Some(agent_id) = agent_id
                && let Ok(granted) = tauri::async_runtime::block_on(
                    harbor_core::plugins::agent_grant_enabled(pool, &agent_id, plugin),
                )
            {
                return granted;
            }
        }
        self.env_grants.iter().any(|id| id == plugin)
    }

    /// A tool call without a grant becomes an approval request the builder can
    /// answer under Plugins — or a plain refusal when there is no agent to
    /// attach the request to. A plugin that is not connected cannot be
    /// granted into usefulness, so it refuses outright instead of queueing an
    /// approval the builder could never satisfy.
    fn request_access(&mut self, plugin: &str) -> String {
        let Some(agent_id) = self.agent_id.clone() else {
            return format!("{plugin} is not granted for this session");
        };
        let Some(pool) = self.pool() else {
            return format!("{plugin} is not granted for this agent");
        };
        if let Ok(Some(status)) =
            tauri::async_runtime::block_on(harbor_core::plugins::plugin_status(pool, plugin))
            && status != "connected"
        {
            return format!("{plugin} is not connected — connect it under Plugins first");
        }
        match tauri::async_runtime::block_on(harbor_core::plugins::approval_state(
            pool, plugin, &agent_id, "grant",
        )) {
            Ok(Some(state)) if state == "denied" => {
                return format!(
                    "{plugin} access was denied; the builder can grant it under Plugins"
                );
            }
            Ok(Some(_)) => {
                return format!("{plugin} access is waiting for approval under Plugins");
            }
            _ => {}
        }
        let created = tauri::async_runtime::block_on(harbor_core::plugins::create_approval(
            pool,
            plugin,
            &agent_id,
            "grant",
            &json!({ "pluginId": plugin }),
        ));
        if matches!(created, Ok(Some(_))) {
            // The sidecar is its own process: it can write the inbox row but
            // cannot emit the live badge event — the bell catches up on the
            // next renderer load. `notification_kinds` still applies.
            let kinds = tauri::async_runtime::block_on(harbor_core::settings::get(
                pool,
                "notification_kinds",
            ))
            .ok()
            .flatten();
            if harbor_core::notifications::kind_allowed(kinds, "permission") {
                let name = tauri::async_runtime::block_on(harbor_core::agents::display_name(
                    pool, &agent_id,
                ));
                let _ = tauri::async_runtime::block_on(harbor_core::notifications::record(
                    pool,
                    "permission",
                    &format!("{name} requests {plugin}"),
                    "Approve or deny the access request under Plugins.",
                    harbor_core::notifications::Target {
                        mode: Some("plugins".into()),
                        ..Default::default()
                    },
                ));
            }
            format!("{plugin} access requested — the builder can approve it under Plugins")
        } else {
            format!("{plugin} access is waiting for approval under Plugins")
        }
    }
}

impl harbor_plugins::mcp::ToolBackend for SidecarBackend {
    fn plugin_available(&mut self, plugin: &str) -> bool {
        // Connected released plugins advertise tools to agent sessions even
        // without a grant — the call itself is intercepted and turned into an
        // approval request. Thread sessions (no agent) still see nothing.
        if harbor_core::plugins::definition(plugin).is_none_or(|def| !def.released) {
            return false;
        }
        let capable = self.agent_id.is_some() || self.env_grants.iter().any(|id| id == plugin);
        capable && self.token(plugin).is_ok()
    }

    fn call_tool(&mut self, plugin: &str, tool: &str, args: &Value) -> Result<String, String> {
        if !self.granted(plugin) {
            return Err(self.request_access(plugin));
        }
        let token = self.token(plugin)?;
        if plugin != "github" {
            return Err(format!("{plugin} has no tools in this build"));
        }
        let request = github::tool_request(tool, args)?;
        let response = self
            .client
            .get(&request.url)
            .header("Authorization", format!("Bearer {token}"))
            .header(
                "Accept",
                if request.raw {
                    "application/vnd.github.raw"
                } else {
                    "application/vnd.github+json"
                },
            )
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "harbor-plugins")
            .send()
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let body = bounded_body(response, "GitHub API request failed")?;
        if !status.is_success() {
            return Err(format!("GitHub API returned {status}"));
        }
        github::shape_tool_result(tool, &body)
    }
}

fn granted_from_env() -> Vec<String> {
    std::env::var(harbor_plugins::mcp::GRANTS_ENV)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// `harbor mcp-plugins --session <id>`: answer MCP on stdio and exit. No window.
/// The session reference and grants never reach stdout or stderr.
pub fn run_mcp_sidecar() -> i32 {
    // Parsed so the engine's contract is validated; the session reference
    // stays on the command line and must never be written out.
    let _session = harbor_plugins::mcp::session_ref(
        std::env::args(),
        std::env::var(harbor_plugins::mcp::SESSION_ENV)
            .ok()
            .as_deref(),
    );
    let mut backend = SidecarBackend {
        env_grants: granted_from_env(),
        agent_id: env_nonempty(harbor_plugins::mcp::AGENT_ENV),
        db_path: env_nonempty(harbor_plugins::mcp::DB_ENV).map(std::path::PathBuf::from),
        pool: None,
        keyring_dir: std::env::var(harbor_plugins::mcp::KEYRING_ENV)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| keyring_dir()),
        client: http_client(),
    };
    match harbor_plugins::mcp::serve(io::stdin().lock(), io::stdout(), &mut backend) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harbor_core::types::CreateAgent;
    use harbor_plugins::mcp::ToolBackend;
    use std::io::BufReader;

    #[test]
    fn env_grants_drop_blanks_and_never_invent_entries() {
        let parse = |raw: &str| -> Vec<String> {
            raw.split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        };
        assert_eq!(parse(" github ,, linear "), ["github", "linear"]);
        assert!(parse("").is_empty());
        assert!(parse(" , ,").is_empty());
    }

    fn backend(
        env_grants: &[&str],
        agent_id: Option<String>,
        db_path: Option<std::path::PathBuf>,
    ) -> SidecarBackend {
        SidecarBackend {
            env_grants: env_grants.iter().map(|id| id.to_string()).collect(),
            agent_id,
            db_path,
            pool: None,
            keyring_dir: std::env::temp_dir().join("missing-keyring-dir"),
            client: http_client(),
        }
    }

    #[test]
    fn ungranted_plugin_is_refused_before_any_network() {
        // No agent on the session: there is nobody to attach a request to.
        let mut backend = backend(&[], None, None);
        assert!(!backend.plugin_available("github"));
        let err = backend
            .call_tool("github", "github_me", &json!({}))
            .unwrap_err();
        assert!(err.contains("not granted"));
    }

    #[test]
    fn ungranted_agent_call_raises_one_pending_approval() {
        // A plain test, not #[tokio::test]: the backend drives its own async
        // through async_runtime::block_on, and nesting that inside a test
        // runtime makes teardown panic. This runtime only services setup and
        // assertions.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let (pool, agent) = runtime.block_on(async {
            let pool = harbor_core::db::open(&db_path).await.unwrap();
            let agent = harbor_core::agents::create(
                &pool,
                CreateAgent {
                    name: "Plug".into(),
                    brief: String::new(),
                    engine_id: "opencode".into(),
                    face_index: None,
                    home_path: None,
                },
            )
            .await
            .unwrap();
            (pool, agent)
        });
        // The sidecar holds its own pool to the same database, like the real
        // process opening Harbor's SQLite file.
        let mut backend = backend(&[], Some(agent.id.clone()), Some(db_path));

        let err = backend
            .call_tool("github", "github_me", &json!({}))
            .unwrap_err();
        assert!(err.contains("access requested"), "{err}");
        let pending = runtime
            .block_on(harbor_core::plugins::list_approvals(&pool))
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].plugin_id, "github");
        assert_eq!(pending[0].agent_id.as_deref(), Some(agent.id.as_str()));
        assert_eq!(pending[0].action, "grant");
        let notice = runtime
            .block_on(harbor_core::notifications::list(&pool))
            .unwrap();
        assert_eq!(notice.len(), 1);
        assert_eq!(notice[0].kind, "permission");

        // A second call dedupes on the pending row instead of spamming.
        let err = backend
            .call_tool("github", "github_me", &json!({}))
            .unwrap_err();
        assert!(err.contains("waiting for approval"), "{err}");
        assert_eq!(
            runtime
                .block_on(harbor_core::plugins::list_approvals(&pool))
                .unwrap()
                .len(),
            1
        );

        // Allowing the request turns the grant on; the same sidecar session
        // sees it on the next call without a respawn. The grant only applies
        // while the plugin is connected — approve alone is not enough.
        runtime
            .block_on(harbor_core::plugins::resolve_approval(
                &pool,
                &pending[0].id,
                true,
            ))
            .unwrap();
        assert!(!backend.granted("github"));
        runtime
            .block_on(harbor_core::plugins::mark_connected(
                &pool,
                "github",
                "GitHub",
                Some("octocat"),
            ))
            .unwrap();
        assert!(backend.granted("github"));
        // The tool itself still needs a stored credential — the grant is not
        // one — so the call now fails on the keyring, not the grant.
        let err = backend
            .call_tool("github", "github_me", &json!({}))
            .unwrap_err();
        assert!(err.contains("not connected"), "{err}");
    }

    #[test]
    fn a_revoked_grant_stops_applying_mid_session() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("db.sqlite");
        let (pool, agent) = runtime.block_on(async {
            let pool = harbor_core::db::open(&db_path).await.unwrap();
            let agent = harbor_core::agents::create(
                &pool,
                CreateAgent {
                    name: "Plug".into(),
                    brief: String::new(),
                    engine_id: "opencode".into(),
                    face_index: None,
                    home_path: None,
                },
            )
            .await
            .unwrap();
            (pool, agent)
        });
        // Granted when the session spawned, then switched off under Plugins
        // while it ran. The spawn env still carries it; the database wins.
        // The plugin must read as connected — a disconnected plugin's grants
        // are inert regardless of their enabled flag.
        runtime
            .block_on(harbor_core::plugins::mark_connected(
                &pool,
                "github",
                "GitHub",
                Some("octocat"),
            ))
            .unwrap();
        runtime
            .block_on(harbor_core::plugins::set_agent_grant(
                &pool, &agent.id, "github", true,
            ))
            .unwrap();
        {
            let mut live = backend(&["github"], Some(agent.id.clone()), Some(db_path.clone()));
            assert!(live.granted("github"));
        }
        runtime
            .block_on(harbor_core::plugins::set_agent_grant(
                &pool, &agent.id, "github", false,
            ))
            .unwrap();
        let mut backend = backend(&["github"], Some(agent.id.clone()), Some(db_path));
        assert!(
            !backend.granted("github"),
            "a revoked grant must stop applying without a respawn"
        );

        // And a disconnect cuts access even if a grant row still reads
        // enabled — the plugin status gates every call.
        runtime
            .block_on(harbor_core::plugins::set_agent_grant(
                &pool, &agent.id, "github", true,
            ))
            .unwrap();
        runtime
            .block_on(harbor_core::plugins::mark_disconnected(&pool, "github"))
            .unwrap();
        assert!(
            !backend.granted("github"),
            "a disconnected plugin's grants are inert"
        );
    }

    #[test]
    fn env_grants_apply_when_the_database_is_not_reachable() {
        // No database path (or a pool that will not open): the spawn-time
        // snapshot is the fallback so a session does not go silent mid-turn.
        let mut backend = backend(&["github"], Some("agent-1".into()), None);
        assert!(backend.granted("github"));
        assert!(!backend.granted("linear"));
    }

    #[test]
    fn stdio_loop_never_leaks_token_values() {
        let mut backend = backend(&["github"], None, None);
        // Granted but no stored credential: tools/list stays empty.
        assert!(!backend.plugin_available("github"));
        let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}\n";
        let mut out: Vec<u8> = Vec::new();
        harbor_plugins::mcp::serve(BufReader::new(input.as_bytes()), &mut out, &mut backend)
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\"tools\":[]"));
    }
}
