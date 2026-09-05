use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AcpError;
use crate::spawn::{McpServer, SpawnSpec};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitializeCaps {
    pub resume: bool,
    pub load_session: bool,
    pub additional_directories: bool,
    pub auth_methods: Vec<String>,
    pub config_options: Vec<ConfigOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigOption {
    pub id: String,
    pub category: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeKind {
    Resumed,
    LoadedNoReplayPersist,
    FreshWithBanner,
    Fresh,
}

/// Resume policy: resume → load → new. Never treat loadSession as resume.
pub fn resume_or_new(stored: Option<&str>, caps: &InitializeCaps) -> ResumeKind {
    match stored {
        Some(_) if caps.resume => ResumeKind::Resumed,
        Some(_) if caps.load_session => ResumeKind::LoadedNoReplayPersist,
        Some(_) => ResumeKind::FreshWithBanner,
        None => ResumeKind::Fresh,
    }
}

pub fn method_for(kind: ResumeKind) -> &'static str {
    match kind {
        ResumeKind::Resumed => "session/resume",
        ResumeKind::LoadedNoReplayPersist => "session/load",
        ResumeKind::FreshWithBanner | ResumeKind::Fresh => "session/new",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub cwd: String,
    pub mcp_servers: Vec<McpServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_directories: Option<Vec<String>>,
}

pub fn session_params(
    kind: ResumeKind,
    stored: Option<String>,
    spec: &SpawnSpec,
    extra_roots: &[String],
    caps: &InitializeCaps,
) -> SessionParams {
    let additional_directories = if caps.additional_directories {
        Some(extra_roots.to_vec())
    } else {
        None
    };
    SessionParams {
        session_id: match kind {
            ResumeKind::Resumed | ResumeKind::LoadedNoReplayPersist => stored,
            ResumeKind::FreshWithBanner | ResumeKind::Fresh => None,
        },
        cwd: spec.cwd.clone(),
        mcp_servers: spec.mcp_servers.clone(),
        additional_directories,
    }
}

pub fn should_drop_session_update(kind: ResumeKind, replay: bool) -> bool {
    matches!(kind, ResumeKind::LoadedNoReplayPersist) && replay
}

pub fn parse_initialize_caps(result: &Value) -> InitializeCaps {
    let agent = result
        .get("agentCapabilities")
        .cloned()
        .unwrap_or(Value::Null);
    let session = agent
        .get("sessionCapabilities")
        .cloned()
        .unwrap_or(Value::Null);
    let auth = result
        .get("authMethods")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let options = result
        .get("configOptions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    InitializeCaps {
        resume: session.get("resume").is_some(),
        load_session: agent.get("loadSession").and_then(Value::as_bool) == Some(true),
        additional_directories: session.get("additionalDirectories").is_some(),
        auth_methods: auth
            .iter()
            .filter_map(|value| value.get("id").and_then(Value::as_str).map(str::to_string))
            .collect(),
        config_options: options
            .iter()
            .filter_map(|value| {
                Some(ConfigOption {
                    id: value.get("id")?.as_str()?.to_string(),
                    category: value
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or("model")
                        .to_string(),
                })
            })
            .collect(),
    }
}

pub struct AcpHostSession {
    pub session_id: Option<String>,
    pub engine_id: String,
    pub cwd: String,
}

impl AcpHostSession {
    pub async fn start(spec: SpawnSpec) -> Result<Self, AcpError> {
        Ok(Self {
            session_id: None,
            engine_id: spec.engine_id,
            cwd: spec.cwd,
        })
    }

    pub async fn resume_or_new(&mut self, stored: Option<String>) -> Result<ResumeKind, AcpError> {
        let caps = InitializeCaps::default();
        let kind = resume_or_new(stored.as_deref(), &caps);
        self.session_id = match kind {
            ResumeKind::Resumed | ResumeKind::LoadedNoReplayPersist => stored,
            ResumeKind::FreshWithBanner | ResumeKind::Fresh => None,
        };
        Ok(kind)
    }
}
