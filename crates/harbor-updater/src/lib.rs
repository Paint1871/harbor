//! GitHub Releases + baked minisign public key. Unsigned artifacts are refused.

pub const PLACEHOLDER_KEY: &str = include_str!("../../../apps/desktop/src-tauri/minisign.pub");

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("unsigned or tampered update refused")]
    Unsigned,
    #[error("placeholder public key cannot authorize an update")]
    PlaceholderKey,
}

pub fn public_key() -> &'static str {
    PLACEHOLDER_KEY
}

pub fn verify_release(signature: &str, key: &str) -> Result<(), UpdateError> {
    if key.contains("placeholder") || !key.contains("untrusted comment: minisign public key") {
        return Err(UpdateError::PlaceholderKey);
    }
    if !signature.contains("untrusted comment: minisign signature") {
        return Err(UpdateError::Unsigned);
    }
    Err(UpdateError::Unsigned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_never_authorizes() {
        assert_eq!(
            verify_release("untrusted comment: minisign signature\nRWS", public_key()),
            Err(UpdateError::PlaceholderKey)
        );
    }
}
