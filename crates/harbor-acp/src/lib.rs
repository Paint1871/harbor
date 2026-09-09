//! ACP v1 client. Protocol v2 is out of scope for Harbor 0.1.0.

pub mod permissions;
pub mod session;
pub mod spawn;
pub mod transport;

pub use permissions::{PermissionKind, map_permission_kind, permission_outcome};
pub use session::{
    ConfigOption, ConfigSource, ConfigValue, InitializeCaps, ResumeKind, parse_config_options,
    resume_or_new, set_option_call,
};
pub use spawn::{McpServer, SpawnSpec};
pub use transport::PermissionHook;

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Protocol(&'static str),
    #[error("unimplemented: {0}")]
    Unimplemented(&'static str),
}
