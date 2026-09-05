use harbor_acp::{
    PermissionKind, map_permission_kind,
    permissions::permission_outcome,
    session::{
        InitializeCaps, ResumeKind, method_for, parse_initialize_caps, resume_or_new,
        session_params, should_drop_session_update,
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
        "configOptions": [{ "id": "model", "category": "model" }, { "id": "mode", "category": "mode" }]
    }));
    assert!(caps.additional_directories);
    assert_eq!(caps.config_options.len(), 2);
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
