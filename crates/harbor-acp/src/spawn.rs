use serde::{Deserialize, Serialize};

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
                name: "HARBOR_PLUGIN_SESSION".into(),
                value: session_ref.into(),
            }],
        }
    }
}
