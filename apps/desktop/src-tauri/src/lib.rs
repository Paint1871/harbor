//! Desktop host bootstrap. Tauri window setup follows in PR-03.

/// The application version shared with the core.
pub fn version() -> &'static str {
    harbor_core::version()
}
