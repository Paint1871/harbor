use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EngineSpec {
    pub id: String,
    pub display_name: String,
    pub binaries: Vec<String>,
    pub acp_args: Option<Vec<String>>,
    pub pty_args: Vec<String>,
    pub supports_terminal: bool,
    pub adapter_package: Option<String>,
    pub min_version: Option<String>,
    pub last_handshake: Option<String>,
    pub auth_hint: String,
    pub chat_mode: String,
    #[serde(default)]
    pub workspace_settings_arg: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Workspace {
    pub id: String,
    pub folder: String,
    pub title: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSetup {
    pub additional_terminals: u8,
    #[serde(default)]
    pub browser_preview: bool,
    #[serde(default)]
    pub thread_pane: bool,
    #[serde(default)]
    pub terminal_engine_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DetectedEngine {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub status: String,
    pub supports_chat: bool,
    pub supports_terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaneLayout {
    Leaf {
        #[serde(rename = "paneId", alias = "pane_id")]
        pane_id: String,
    },
    Split {
        dir: String,
        ratio: f64,
        a: Box<PaneLayout>,
        b: Box<PaneLayout>,
    },
    Tabs {
        active: i32,
        kids: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRecord {
    pub id: String,
    pub workspace_id: Option<String>,
    pub title: String,
    pub engine_id: String,
    pub pinned: bool,
    pub unread: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentTrailing {
    Running {
        n: i32,
    },
    NeedsYou,
    LastSpoke {
        at: i64,
    },
    #[default]
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub brief: String,
    pub engine_id: String,
    pub face_index: i32,
    pub pinned: bool,
    #[serde(default)]
    pub home_path: String,
    #[serde(default)]
    pub messaging: bool,
    #[serde(default)]
    pub last_line: Option<String>,
    #[serde(default)]
    pub trailing: AgentTrailing,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgent {
    pub name: String,
    pub brief: String,
    pub engine_id: String,
    #[serde(default)]
    pub face_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgent {
    pub id: String,
    pub name: Option<String>,
    pub brief: Option<String>,
    pub engine_id: Option<String>,
    pub face_index: Option<i32>,
    pub pinned: Option<bool>,
    #[serde(default)]
    pub home_path: Option<String>,
    #[serde(default)]
    pub messaging: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentChat {
    pub id: String,
    pub agent_id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Memory {
    pub id: String,
    pub body: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SearchHit {
    pub chat_id: String,
    pub prose: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginRow {
    pub id: String,
    pub display_name: String,
    pub status: String,
    pub account_label: Option<String>,
    pub description: String,
    pub category: String,
    pub auth_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginGrant {
    pub plugin_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PluginApproval {
    pub id: String,
    pub plugin_id: String,
    pub agent_id: Option<String>,
    pub action: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RestoredPane {
    pub id: String,
    pub kind: String,
    pub paused: bool,
    #[serde(default)]
    pub engine_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTab {
    pub id: String,
    pub workspace_id: String,
    pub layout: PaneLayout,
    pub panes: Vec<RestoredPane>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FileDiff {
    pub path: String,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateStatus {
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ContentPart {
    pub r#type: String,
    pub text: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PaneState {
    pub kind: String,
    pub cwd: Option<String>,
    pub paused: Option<bool>,
    #[serde(default)]
    pub engine_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub created_at: i64,
    /// Where clicking the row should take the builder.
    pub mode: Option<String>,
    pub workspace_id: Option<String>,
    pub pane_id: Option<String>,
    pub session_ref: Option<String>,
}
