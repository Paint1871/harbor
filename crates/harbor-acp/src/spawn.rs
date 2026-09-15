use serde::{Deserialize, Serialize};

/// Env passed to the harbor-plugins MCP sidecar. Session only; never a token.
pub const PLUGIN_SESSION_ENV: &str = "HARBOR_PLUGIN_SESSION";

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
    pub fn harbor_plugins(harbor_bin: &str, session_ref: &str) -> McpServer {
        McpServer {
            name: "harbor-plugins".into(),
            command: harbor_bin.into(),
            args: vec!["mcp-plugins".into(), "--session".into(), session_ref.into()],
            env: vec![EnvVariable {
                name: PLUGIN_SESSION_ENV.into(),
                value: session_ref.into(),
            }],
        }
    }
}

/// MCP servers attached to a live ACP `SpawnSpec`.
///
/// `harbor_bin` is `current_exe()` as a lossy string. An empty path means the
/// host could not resolve itself; leave `mcp_servers` empty rather than guess.
pub fn plugin_mcp_servers(harbor_bin: &str, session_ref: &str) -> Vec<McpServer> {
    if harbor_bin.is_empty() {
        Vec::new()
    } else {
        vec![SpawnSpec::harbor_plugins(harbor_bin, session_ref)]
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
        let servers = plugin_mcp_servers("/usr/bin/harbor", "sess-live");
        assert_eq!(servers.len(), 1);
        let server = &servers[0];
        assert_eq!(server.name, "harbor-plugins");
        assert_eq!(server.command, "/usr/bin/harbor");
        assert_eq!(server.args, vec!["mcp-plugins", "--session", "sess-live"]);
        assert_eq!(server.env.len(), 1);
        assert_eq!(server.env[0].name, PLUGIN_SESSION_ENV);
        assert_eq!(server.env[0].value, "sess-live");

        let encoded = serde_json::to_value(server).unwrap();
        assert!(encoded["env"].is_array());
        assert!(!encoded["env"].is_object());
        for env in &server.env {
            assert!(!env_name_looks_like_secret(&env.name), "{}", env.name);
            assert!(!looks_tokenish(&env.value), "{}", env.value);
        }
        assert!(!looks_tokenish(&encoded.to_string()));
        assert!(plugin_mcp_servers("", "sess-live").is_empty());
    }
}
