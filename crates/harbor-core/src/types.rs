use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Workspace {
    pub id: String,
    pub folder: String,
    pub title: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DetectedEngine {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub status: String,
    pub supports_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaneLayout {
    Leaf {
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub brief: String,
    pub engine_id: String,
    pub face_index: i32,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgent {
    pub name: String,
    pub brief: String,
    pub engine_id: String,
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
}
