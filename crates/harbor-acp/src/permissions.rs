#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

pub fn map_permission_kind(kind: &str) -> Option<(PermissionKind, &'static str)> {
    match kind {
        "allow_once" => Some((PermissionKind::AllowOnce, "Allow")),
        "allow_always" => Some((PermissionKind::AllowAlways, "Allow for session")),
        "reject_once" | "reject_always" => Some((PermissionKind::RejectOnce, "Deny")),
        _ => None,
    }
}

pub fn permission_outcome(option_id: Option<&str>, cancelled: bool) -> serde_json::Value {
    if cancelled {
        serde_json::json!({ "outcome": "cancelled" })
    } else if let Some(option_id) = option_id {
        serde_json::json!({ "outcome": "selected", "optionId": option_id })
    } else {
        serde_json::json!({ "outcome": "cancelled" })
    }
}
