use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DictationState {
    Idle,
    Listening,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationEvent {
    pub state: DictationState,
    pub copy: &'static str,
}

pub fn begin() -> DictationEvent {
    DictationEvent {
        state: DictationState::Listening,
        copy: "Listening · release fn to send",
    }
}

pub fn silence_timeout() -> DictationEvent {
    DictationEvent {
        state: DictationState::Error,
        copy: "The microphone isn't delivering audio",
    }
}

pub fn ignore_phantom_tap(duration_ms: u64) -> bool {
    duration_ms < 150
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_copy_and_anti_accidental() {
        assert_eq!(begin().copy, "Listening · release fn to send");
        assert_eq!(
            silence_timeout().copy,
            "The microphone isn't delivering audio"
        );
        assert!(ignore_phantom_tap(120));
        assert!(!ignore_phantom_tap(200));
    }
}
