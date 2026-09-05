//! Layout restore: panes and tab order, never processes or scrollback.

pub fn restored_terminal_paused() -> bool {
    true
}

pub fn spawn_acp_on_cold_start() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_start_does_not_spawn_or_resume_processes() {
        assert!(restored_terminal_paused());
        assert!(!spawn_acp_on_cold_start());
    }
}
