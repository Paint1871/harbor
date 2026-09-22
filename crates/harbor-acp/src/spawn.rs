use serde::{Deserialize, Serialize};

/// Env passed to the harbor-plugins MCP sidecar. Session only; never a token.
pub const PLUGIN_SESSION_ENV: &str = "HARBOR_PLUGIN_SESSION";
/// Comma-separated plugin ids granted for the session's agent. Ids, never tokens.
pub const PLUGIN_GRANTS_ENV: &str = "HARBOR_PLUGIN_GRANTS";
/// The file-fallback credential directory; the sidecar tries the OS keyring first.
pub const PLUGIN_KEYRING_ENV: &str = "HARBOR_KEYRING_DIR";
/// Agent the session belongs to; threads have none, and the sidecar then
/// cannot raise approval requests because grants are per-agent.
pub const PLUGIN_AGENT_ENV: &str = "HARBOR_PLUGIN_AGENT";
/// Harbor's SQLite path, so the sidecar can re-check grants live and record
/// approval requests. A path, never its contents.
pub const PLUGIN_DB_ENV: &str = "HARBOR_DB";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvVariable {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    /// ACP v1: array of {name,value}, never a map.
    pub env: Vec<EnvVariable>,
}

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub engine_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub mcp_servers: Vec<McpServer>,
}

impl SpawnSpec {
    /// `granted` lists the plugin ids the session's agent may use; the sidecar
    /// resolves credentials itself. `keyring_dir` is the fallback store path.
    /// `agent_id` + `db_path` let the sidecar record approval requests and
    /// pick up grants the builder approves mid-session.
    pub fn harbor_plugins(
        harbor_bin: &str,
        session_ref: &str,
        agent_id: Option<&str>,
        granted: &[String],
        keyring_dir: &str,
        db_path: &str,
    ) -> McpServer {
        let mut env = vec![
            EnvVariable {
                name: PLUGIN_SESSION_ENV.into(),
                value: session_ref.into(),
            },
            EnvVariable {
                name: PLUGIN_GRANTS_ENV.into(),
                value: granted.join(","),
            },
            EnvVariable {
                name: PLUGIN_KEYRING_ENV.into(),
                value: keyring_dir.into(),
            },
        ];
        if let Some(agent_id) = agent_id {
            env.push(EnvVariable {
                name: PLUGIN_AGENT_ENV.into(),
                value: agent_id.into(),
            });
            env.push(EnvVariable {
                name: PLUGIN_DB_ENV.into(),
                value: db_path.into(),
            });
        }
        McpServer {
            name: "harbor-plugins".into(),
            command: harbor_bin.into(),
            args: vec!["mcp-plugins".into(), "--session".into(), session_ref.into()],
            env,
        }
    }
}

/// MCP servers attached to a live ACP `SpawnSpec`.
///
/// `harbor_bin` is `current_exe()` as a lossy string. An empty path means the
/// host could not resolve itself; leave `mcp_servers` empty rather than guess.
pub fn plugin_mcp_servers(
    harbor_bin: &str,
    session_ref: &str,
    agent_id: Option<&str>,
    granted: &[String],
    keyring_dir: &str,
    db_path: &str,
) -> Vec<McpServer> {
    if harbor_bin.is_empty() {
        Vec::new()
    } else {
        vec![SpawnSpec::harbor_plugins(
            harbor_bin,
            session_ref,
            agent_id,
            granted,
            keyring_dir,
            db_path,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looks_tokenish(value: &str) -> bool {
        value.starts_with("ghu_")
            || value.starts_with("gho_")
            || value.starts_with("ghp_")
            || value.starts_with("github_pat_")
            || value.starts_with("sk-")
            || value.contains("Bearer ")
    }

    fn env_name_looks_like_secret(name: &str) -> bool {
        let upper = name.to_ascii_uppercase();
        upper.contains("TOKEN")
            || upper.contains("SECRET")
            || upper.contains("PASSWORD")
            || upper.contains("API_KEY")
    }

    #[test]
    fn live_spawn_spec_includes_harbor_plugins_without_tokens() {
        let granted = vec!["github".to_string()];
        let servers = plugin_mcp_servers(
            "/usr/bin/harbor",
            "sess-live",
            Some("agent-7"),
            &granted,
            "/data/harbor/keyring",
            "/data/harbor/harbor.sqlite",
        );
        assert_eq!(servers.len(), 1);
        let server = &servers[0];
        assert_eq!(server.name, "harbor-plugins");
        assert_eq!(server.command, "/usr/bin/harbor");
        assert_eq!(server.args, vec!["mcp-plugins", "--session", "sess-live"]);
        assert_eq!(server.env.len(), 5);
        assert_eq!(server.env[0].name, PLUGIN_SESSION_ENV);
        assert_eq!(server.env[0].value, "sess-live");
        assert_eq!(server.env[1].name, PLUGIN_GRANTS_ENV);
        assert_eq!(server.env[1].value, "github");
        assert_eq!(server.env[2].name, PLUGIN_KEYRING_ENV);
        assert_eq!(server.env[2].value, "/data/harbor/keyring");
        assert_eq!(server.env[3].name, PLUGIN_AGENT_ENV);
        assert_eq!(server.env[3].value, "agent-7");
        assert_eq!(server.env[4].name, PLUGIN_DB_ENV);
        assert_eq!(server.env[4].value, "/data/harbor/harbor.sqlite");

        // A session without an agent (a folder thread) carries no agent or db
        // handle — it cannot raise approval requests.
        let thread =
            &plugin_mcp_servers("/usr/bin/harbor", "thread-1", None, &granted, "/k", "/db")[0];
        assert_eq!(thread.env.len(), 3);

        let encoded = serde_json::to_value(server).unwrap();
        assert!(encoded["env"].is_array());
        assert!(!encoded["env"].is_object());
        for env in &server.env {
            assert!(!env_name_looks_like_secret(&env.name), "{}", env.name);
            assert!(!looks_tokenish(&env.value), "{}", env.value);
        }
        assert!(!looks_tokenish(&encoded.to_string()));
        assert!(plugin_mcp_servers("", "sess-live", Some("a"), &granted, "/k", "/db").is_empty());
    }
}
