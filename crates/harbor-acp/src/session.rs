use std::process::ChildStdin;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::AcpError;
use crate::spawn::{McpServer, SpawnSpec};
use crate::transport::PermissionHook;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitializeCaps {
    pub resume: bool,
    pub load_session: bool,
    pub additional_directories: bool,
    pub auth_methods: Vec<String>,
    pub config_options: Vec<ConfigOption>,
}

/// Where an option came from, because that decides how it is set again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConfigSource {
    /// An agent's own `configOptions` list, set with `session/set_config_option`.
    #[default]
    Config,
    /// The protocol's `models` block, set with `session/set_model`.
    Model,
    /// The protocol's `modes` block, set with `session/set_mode`.
    Mode,
}

/// One knob an agent exposes — "Model", "Session Mode" — with the values it
/// accepts. Agents disagree about where to put these: some send a
/// `configOptions` list, others the protocol's own `models` and `modes`
/// blocks, and one sends both. All of them are read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    pub category: String,
    pub current_value: Option<String>,
    pub values: Vec<ConfigValue>,
    #[serde(default)]
    pub source: ConfigSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValue {
    pub value: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `{ currentXId, availableXs: [{ idKey, name, description }] }` — the shape the
/// protocol uses for both models and modes.
struct StateBlock {
    block: &'static str,
    current_key: &'static str,
    list_key: &'static str,
    id_key: &'static str,
    option_id: &'static str,
    option_name: &'static str,
    category: &'static str,
    source: ConfigSource,
}

const MODEL_BLOCK: StateBlock = StateBlock {
    block: "models",
    current_key: "currentModelId",
    list_key: "availableModels",
    id_key: "modelId",
    option_id: "model",
    option_name: "Model",
    category: "model",
    source: ConfigSource::Model,
};

const MODE_BLOCK: StateBlock = StateBlock {
    block: "modes",
    current_key: "currentModeId",
    list_key: "availableModes",
    id_key: "id",
    option_id: "mode",
    option_name: "Mode",
    category: "mode",
    source: ConfigSource::Mode,
};

fn parse_state_block(result: &Value, spec: &StateBlock) -> Option<ConfigOption> {
    let state = result.get(spec.block)?;
    let values: Vec<ConfigValue> = state
        .get(spec.list_key)?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let id = entry.get(spec.id_key)?.as_str()?.to_string();
            Some(ConfigValue {
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&id)
                    .to_string(),
                value: id,
                description: entry
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect();
    if values.is_empty() {
        return None;
    }
    Some(ConfigOption {
        id: spec.option_id.to_string(),
        name: spec.option_name.to_string(),
        category: spec.category.to_string(),
        current_value: state
            .get(spec.current_key)
            .and_then(Value::as_str)
            .map(str::to_string),
        values,
        source: spec.source,
    })
}

pub fn parse_config_options(result: &Value) -> Vec<ConfigOption> {
    let declared = parse_declared_options(result);
    if !declared.is_empty() {
        return declared;
    }
    [
        parse_state_block(result, &MODEL_BLOCK),
        parse_state_block(result, &MODE_BLOCK),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn parse_declared_options(result: &Value) -> Vec<ConfigOption> {
    result
        .get("configOptions")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let id = option.get("id")?.as_str()?.to_string();
                    let category = option
                        .get("category")
                        .and_then(Value::as_str)
                        .unwrap_or("model")
                        .to_string();
                    Some(ConfigOption {
                        name: option
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or(&id)
                            .to_string(),
                        id,
                        category,
                        current_value: option
                            .get("currentValue")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        values: option
                            .get("options")
                            .and_then(Value::as_array)
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(|value| {
                                        let id = value.get("value")?.as_str()?.to_string();
                                        Some(ConfigValue {
                                            name: value
                                                .get("name")
                                                .and_then(Value::as_str)
                                                .unwrap_or(&id)
                                                .to_string(),
                                            value: id,
                                            description: value
                                                .get("description")
                                                .and_then(Value::as_str)
                                                .map(str::to_string),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        source: ConfigSource::Config,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
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
    InitializeCaps {
        resume: session.get("resume").is_some(),
        load_session: agent.get("loadSession").and_then(Value::as_bool) == Some(true),
        additional_directories: session.get("additionalDirectories").is_some(),
        auth_methods: auth
            .iter()
            .filter_map(|value| value.get("id").and_then(Value::as_str).map(str::to_string))
            .collect(),
        config_options: parse_config_options(result),
    }
}

pub struct AcpHostSession {
    conn: crate::transport::AcpConn,
    pub session_id: Option<String>,
    pub engine_id: String,
    pub cwd: String,
    pub caps: InitializeCaps,
    pub resume_kind: ResumeKind,
    /// What the agent last told us it accepts. Refreshed on every answer that
    /// carries `configOptions`, because setting one can change the others.
    pub config_options: Vec<ConfigOption>,
}

impl AcpHostSession {
    pub fn connect(spec: SpawnSpec) -> Result<Self, AcpError> {
        let mut conn = crate::transport::AcpConn::spawn(&spec)?;
        let init = conn.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": { "readTextFile": false, "writeTextFile": false },
                    "terminal": false
                },
                "clientInfo": { "name": "harbor", "version": "0.1.0" }
            }),
        )?;
        let caps = parse_initialize_caps(&init);
        // Advertised authentication methods do not mean an existing CLI login is invalid.
        Ok(Self {
            conn,
            session_id: None,
            engine_id: spec.engine_id,
            cwd: spec.cwd.clone(),
            config_options: caps.config_options.clone(),
            caps,
            resume_kind: ResumeKind::Fresh,
        })
    }

    pub fn open_session(
        &mut self,
        stored: Option<String>,
        spec: &SpawnSpec,
        extra_roots: &[String],
    ) -> Result<ResumeKind, AcpError> {
        let kind = resume_or_new(stored.as_deref(), &self.caps);
        let params = session_params(kind, stored.clone(), spec, extra_roots, &self.caps);
        let result = self
            .conn
            .request(method_for(kind), serde_json::to_value(&params)?)?;
        if matches!(kind, ResumeKind::LoadedNoReplayPersist) {
            self.conn.notifications.retain(|note| {
                !should_drop_session_update(
                    kind,
                    note.get("params")
                        .and_then(|p| p.get("sessionUpdate"))
                        .and_then(Value::as_str)
                        == Some("replay"),
                )
            });
        }
        let options = parse_config_options(&result);
        if !options.is_empty() {
            self.config_options = options;
        }
        self.session_id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(stored);
        self.resume_kind = kind;
        Ok(kind)
    }

    pub fn prompt(&mut self, parts: &[Value]) -> Result<Value, AcpError> {
        self.conn.request(
            "session/prompt",
            json!({
                "sessionId": self.session_id,
                "prompt": parts
            }),
        )
    }

    pub fn cancel(&mut self) -> Result<Value, AcpError> {
        self.conn
            .notify("session/cancel", json!({ "sessionId": self.session_id }))?;
        Ok(Value::Null)
    }

    pub fn set_permission_hook(&mut self, hook: PermissionHook) {
        self.conn.permission_hook = Some(hook);
    }

    pub fn stdin_handle(&self) -> Arc<Mutex<ChildStdin>> {
        self.conn.stdin_handle()
    }

    /// Set an option by the route it arrived on. A model that came from the
    /// protocol's `models` block is changed with `session/set_model`; sending
    /// it as a config option would be refused as an unknown id.
    pub fn set_config_option(&mut self, id: &str, value: Value) -> Result<Value, AcpError> {
        let source = self
            .config_options
            .iter()
            .find(|option| option.id == id)
            .map(|option| option.source)
            .unwrap_or_default();
        let (method, params) = match source {
            ConfigSource::Model => (
                "session/set_model",
                json!({ "sessionId": self.session_id, "modelId": value }),
            ),
            ConfigSource::Mode => (
                "session/set_mode",
                json!({ "sessionId": self.session_id, "modeId": value }),
            ),
            ConfigSource::Config => (
                "session/set_config_option",
                json!({ "sessionId": self.session_id, "configId": id, "value": value }),
            ),
        };
        let result = self.conn.request(method, params)?;
        let options = parse_config_options(&result);
        if !options.is_empty() {
            self.config_options = options;
        } else if let Some(chosen) = value.as_str() {
            // set_model and set_mode answer with nothing to re-read, so remember
            // the choice rather than showing the old value back.
            if let Some(option) = self
                .config_options
                .iter_mut()
                .find(|option| option.id == id)
            {
                option.current_value = Some(chosen.to_string());
            }
        }
        Ok(result)
    }

    pub fn notifications(&self) -> &[Value] {
        &self.conn.notifications
    }

    pub fn take_notifications(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.conn.notifications)
    }
}
