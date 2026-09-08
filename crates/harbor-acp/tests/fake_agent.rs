use harbor_acp::{
    PermissionKind, map_permission_kind,
    permissions::permission_outcome,
    session::{
        ConfigSource, InitializeCaps, ResumeKind, method_for, parse_config_options,
        parse_initialize_caps, resume_or_new, session_params, should_drop_session_update,
    },
    spawn::{EnvVariable, McpServer, SpawnSpec},
};
use serde_json::json;

fn spec() -> SpawnSpec {
    SpawnSpec {
        engine_id: "opencode".into(),
        command: "opencode".into(),
        args: vec!["acp".into()],
        cwd: "/tmp/proj".into(),
        mcp_servers: vec![SpawnSpec::harbor_plugins("/usr/bin/harbor", "sess-1")],
    }
}

#[test]
fn resume_only_calls_session_resume_never_load() {
    let caps = InitializeCaps {
        resume: true,
        load_session: false,
        additional_directories: true,
        auth_methods: vec![],
        config_options: vec![],
    };
    let kind = resume_or_new(Some("abc"), &caps);
    assert_eq!(kind, ResumeKind::Resumed);
    assert_eq!(method_for(kind), "session/resume");
    let params = session_params(
        kind,
        Some("abc".into()),
        &spec(),
        &["/tmp/extra".into()],
        &caps,
    );
    let encoded = serde_json::to_value(&params).unwrap();
    assert_eq!(encoded["sessionId"], "abc");
    assert_eq!(encoded["cwd"], "/tmp/proj");
    assert!(encoded["mcpServers"][0]["env"].is_array());
    assert_eq!(
        encoded["mcpServers"][0]["env"][0]["name"],
        "HARBOR_PLUGIN_SESSION"
    );
    assert_eq!(encoded["additionalDirectories"][0], "/tmp/extra");
    assert_ne!(method_for(kind), "session/load");
}

#[test]
fn load_only_drops_replay_updates() {
    let caps = InitializeCaps {
        resume: false,
        load_session: true,
        ..InitializeCaps::default()
    };
    let kind = resume_or_new(Some("abc"), &caps);
    assert_eq!(kind, ResumeKind::LoadedNoReplayPersist);
    assert_eq!(method_for(kind), "session/load");
    assert!(should_drop_session_update(kind, true));
    assert!(!should_drop_session_update(kind, false));
}

#[test]
fn neither_opens_new_with_banner() {
    let kind = resume_or_new(Some("abc"), &InitializeCaps::default());
    assert_eq!(kind, ResumeKind::FreshWithBanner);
    assert_eq!(method_for(kind), "session/new");
}

#[test]
fn never_maps_load_session_to_resume() {
    let caps = parse_initialize_caps(&json!({
        "agentCapabilities": { "loadSession": true }
    }));
    assert!(!caps.resume);
    assert!(caps.load_session);
    assert_eq!(
        resume_or_new(Some("x"), &caps),
        ResumeKind::LoadedNoReplayPersist
    );
}

#[test]
fn permissions_echo_option_id_and_cancel() {
    assert_eq!(
        map_permission_kind("allow_once"),
        Some((PermissionKind::AllowOnce, "Allow"))
    );
    assert_eq!(
        map_permission_kind("allow_always"),
        Some((PermissionKind::AllowAlways, "Allow for session"))
    );
    assert_eq!(map_permission_kind("reject_once").unwrap().1, "Deny");
    assert_eq!(map_permission_kind("reject_always").unwrap().1, "Deny");
    assert_eq!(map_permission_kind("unknown"), None);
    let selected = permission_outcome(Some("opt-allow"), false);
    assert_eq!(selected["outcome"], "selected");
    assert_eq!(selected["optionId"], "opt-allow");
    assert_eq!(permission_outcome(None, true)["outcome"], "cancelled");
}

#[test]
fn config_options_and_additional_directories_are_opt_in() {
    let caps = parse_initialize_caps(&json!({
        "agentCapabilities": {
            "sessionCapabilities": { "additionalDirectories": {} },
            "loadSession": false
        },
        "configOptions": [
            {
                "id": "model",
                "name": "Model",
                "category": "model",
                "currentValue": "opencode/big-pickle",
                "options": [
                    { "value": "opencode/big-pickle", "name": "OpenCode Zen/Big Pickle" },
                    { "value": "forge/kimi-k3", "name": "Forge AI/Kimi K3" }
                ]
            },
            { "id": "mode", "category": "mode" }
        ]
    }));
    assert!(caps.additional_directories);
    assert_eq!(caps.config_options.len(), 2);
    let model = &caps.config_options[0];
    assert_eq!(model.name, "Model");
    assert_eq!(model.current_value.as_deref(), Some("opencode/big-pickle"));
    assert_eq!(model.values.len(), 2);
    assert_eq!(model.values[1].name, "Forge AI/Kimi K3");
    // A bare option keeps its id as its label rather than rendering as blank.
    assert_eq!(caps.config_options[1].name, "mode");
    assert!(caps.config_options[1].values.is_empty());
    let without = parse_initialize_caps(&json!({}));
    let params = session_params(
        ResumeKind::Fresh,
        None,
        &spec(),
        &["/tmp/x".into()],
        &without,
    );
    let encoded = serde_json::to_value(&params).unwrap();
    assert!(encoded.get("additionalDirectories").is_none());
}

/// opencode answers `session/new`, not `initialize`, with its model list.
#[test]
fn config_options_are_read_from_the_session_result_too() {
    let options = parse_config_options(&json!({
        "sessionId": "ses_1",
        "configOptions": [{
            "id": "model",
            "name": "Model",
            "category": "model",
            "currentValue": "opencode/big-pickle",
            "options": [{ "value": "opencode/big-pickle", "name": "OpenCode Zen/Big Pickle" }]
        }]
    }));
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].values[0].name, "OpenCode Zen/Big Pickle");
    assert!(parse_config_options(&json!({ "sessionId": "ses_1" })).is_empty());
}

/// Grok answers with the protocol's own blocks and no configOptions at all.
#[test]
fn model_and_mode_blocks_stand_in_for_config_options() {
    let options = parse_config_options(&json!({
        "sessionId": "ses_1",
        "models": {
            "currentModelId": "grok-4.6",
            "availableModels": [
                { "modelId": "grok-4.6", "name": "Grok 4.6", "description": "Latest frontier model" },
                { "modelId": "grok-4.5", "name": "Grok 4.5" }
            ]
        },
        "modes": {
            "currentModeId": "plan",
            "availableModes": [{ "id": "plan", "name": "Plan" }, { "id": "build", "name": "Build" }]
        }
    }));
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].id, "model");
    assert_eq!(options[0].source, ConfigSource::Model);
    assert_eq!(options[0].current_value.as_deref(), Some("grok-4.6"));
    assert_eq!(
        options[0].values[0].description.as_deref(),
        Some("Latest frontier model")
    );
    assert_eq!(options[1].id, "mode");
    assert_eq!(options[1].source, ConfigSource::Mode);
    assert_eq!(options[1].values.len(), 2);

    // A declared list wins; claude-agent-acp sends both and means the declared one.
    let both = parse_config_options(&json!({
        "modes": { "currentModeId": "plan", "availableModes": [{ "id": "plan", "name": "Plan" }] },
        "configOptions": [{ "id": "mode", "name": "Mode", "category": "mode", "options": [{ "value": "default", "name": "Manual" }] }]
    }));
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].source, ConfigSource::Config);
    assert_eq!(both[0].values[0].name, "Manual");
}

/// Grok says which effort is running and gives no way to change it: no
/// set method exists, and set_model accepts nonsense without complaint.
#[test]
fn a_reported_but_unsettable_effort_is_marked_as_such() {
    let options = parse_config_options(&json!({
        "models": {
            "currentModelId": "grok-4.6",
            "availableModels": [
                {
                    "modelId": "grok-4.6",
                    "name": "Grok 4.6",
                    "_meta": {
                        "supportsReasoningEffort": true,
                        "reasoningEffort": "xhigh",
                        "reasoningEfforts": [
                            { "value": "xhigh", "label": "Extra High Effort" },
                            { "value": "low", "label": "Low Effort" }
                        ]
                    }
                },
                { "modelId": "grok-4.5", "name": "Grok 4.5", "_meta": { "supportsReasoningEffort": true, "reasoningEfforts": [{ "value": "low", "label": "Low Effort" }] } }
            ]
        }
    }));
    assert_eq!(options.len(), 2);
    let effort = &options[1];
    assert_eq!(effort.id, "effort");
    assert!(!effort.settable);
    assert_eq!(effort.current_value.as_deref(), Some("xhigh"));
    // The levels come from the model in use, not the whole catalogue.
    assert_eq!(effort.values.len(), 2);
    assert_eq!(effort.values[0].name, "Extra High Effort");

    // A model that says nothing about effort contributes no chip.
    let quiet = parse_config_options(&json!({
        "models": { "currentModelId": "m", "availableModels": [{ "modelId": "m", "name": "M" }] }
    }));
    assert_eq!(quiet.len(), 1);
    assert!(quiet[0].settable);
}

#[test]
fn mcp_env_is_name_value_array_not_object() {
    let server = McpServer {
        name: "harbor-plugins".into(),
        command: "/bin/harbor".into(),
        args: vec![],
        env: vec![EnvVariable {
            name: "HARBOR_PLUGIN_SESSION".into(),
            value: "ref".into(),
        }],
    };
    let value = serde_json::to_value(&server).unwrap();
    assert!(value["env"].is_array());
    assert!(!value["env"].is_object());
}
