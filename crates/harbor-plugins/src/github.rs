//! GitHub App + Device Flow. No client secret in the binary.

pub struct DeviceStart {
    pub user_code: String,
    pub verification_uri: String,
}

pub fn revoke_means_delete_keyring_only() -> bool {
    true
}
