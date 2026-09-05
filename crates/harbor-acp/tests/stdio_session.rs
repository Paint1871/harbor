use harbor_acp::session::{AcpHostSession, ResumeKind, method_for};
use harbor_acp::spawn::SpawnSpec;

fn spec(fixture: &str) -> SpawnSpec {
    SpawnSpec {
        engine_id: "opencode".into(),
        command: env!("CARGO_BIN_EXE_harbor-fake-agent").into(),
        args: vec![fixture.into()],
        cwd: std::env::temp_dir().display().to_string(),
        mcp_servers: vec![],
    }
}

#[test]
fn stdio_resume_only_opens_session_resume() {
    let spec = spec("resume_only");
    let mut session = AcpHostSession::connect(spec.clone()).unwrap();
    let kind = session
        .open_session(Some("stored".into()), &spec, &[])
        .unwrap();
    assert_eq!(kind, ResumeKind::Resumed);
    assert_eq!(method_for(kind), "session/resume");
    assert_eq!(session.session_id.as_deref(), Some("sess-fixture"));
}

#[test]
fn stdio_load_only_drops_replay_and_prompts() {
    let spec = spec("load_only");
    let mut session = AcpHostSession::connect(spec.clone()).unwrap();
    let kind = session
        .open_session(Some("stored".into()), &spec, &[])
        .unwrap();
    assert_eq!(kind, ResumeKind::LoadedNoReplayPersist);
    let replay = session
        .notifications()
        .iter()
        .any(|note| note["params"]["sessionUpdate"] == "replay");
    assert!(!replay, "replay updates must not persist");
    let result = session
        .prompt(&[serde_json::json!({"type":"text","text":"hi"})])
        .unwrap();
    assert_eq!(result["stopReason"], "end_turn");
}

#[test]
fn stdio_neither_uses_session_new() {
    let spec = spec("neither");
    let mut session = AcpHostSession::connect(spec.clone()).unwrap();
    let kind = session
        .open_session(Some("stored".into()), &spec, &[])
        .unwrap();
    assert_eq!(kind, ResumeKind::FreshWithBanner);
    assert_eq!(method_for(kind), "session/new");
}
