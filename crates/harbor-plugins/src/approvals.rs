#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Allowed,
    Denied,
}

pub fn connection_is_not_grant() -> bool {
    true
}
