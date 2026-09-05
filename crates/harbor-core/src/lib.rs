//! Local application core for Harbor.

/// The version compiled into this crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
