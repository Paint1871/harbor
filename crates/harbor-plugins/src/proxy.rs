//! stdio MCP proxy. Session ref is opaque; tokens stay in-process.

pub fn mcp_error_reauth() -> &'static str {
    "plugin_reauth_required"
}
