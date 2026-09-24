use serde_json::Value;

/// Keys in `config_json` that are host bookkeeping, not engine options.
const HOST_KEYS: &[&str] = &["attachedFiles"];

/// `config_json` mixes engine options (model, mode, effort) with host state
/// like attached files. Only the options belong on a fresh session — replaying
/// `attachedFiles` would ask the engine to set an option it does not have.
pub fn stored_options(config_json: &str) -> Vec<(String, Value)> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(config_json) else {
        return Vec::new();
    };
    map.into_iter()
        .filter(|(key, _)| !HOST_KEYS.contains(&key.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stored_options_skips_host_keys_and_survives_bad_json() {
        let json = json!({
            "model": "claude-sonnet",
            "attachedFiles": ["/tmp/a.txt"],
            "effort": "high"
        });
        let options = stored_options(&json.to_string());
        assert_eq!(options.len(), 2);
        assert!(options.iter().all(|(key, _)| key != "attachedFiles"));
        assert!(stored_options("not json").is_empty());
        assert!(stored_options("[1,2]").is_empty());
        assert!(stored_options("{}").is_empty());
    }
}
