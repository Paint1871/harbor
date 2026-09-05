//! Assertions against the window, capability, and updater files the host loads.

use super::application_data_root;
use serde_json::Value;
use std::{fs, path::PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(manifest_dir().join(path)).unwrap()).unwrap()
}

fn permission_id(value: &Value) -> &str {
    match value {
        Value::String(text) => text,
        Value::Object(object) => object
            .get("identifier")
            .and_then(Value::as_str)
            .unwrap_or(""),
        _ => "",
    }
}

fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| collect_strings(item, out)),
        Value::Object(object) => object.values().for_each(|item| collect_strings(item, out)),
        _ => {}
    }
}

fn window<'a>(config: &'a Value, label: &str) -> &'a Value {
    config["app"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|window| window["label"] == label)
        .unwrap_or_else(|| panic!("missing window {label}"))
}

fn assert_no_shell(permissions: &[Value]) {
    for permission in permissions {
        let id = permission_id(permission);
        assert!(
            !id.starts_with("shell:"),
            "renderer must not receive shell permissions: {id}"
        );
        assert_ne!(id, "shell:allow-all");
        assert_ne!(id, "shell:allow-execute");
        assert_ne!(id, "shell:allow-spawn");
        assert_ne!(id, "shell:default");
    }
}

#[test]
fn main_and_overlay_match_pr03_geometry() {
    let config = read_json("tauri.conf.json");
    assert_eq!(config["productName"], "Harbor");
    assert_eq!(config["identifier"], "app.harbor.desktop");
    assert_eq!(config["app"]["security"]["capabilities"][0], "main");
    assert_eq!(config["app"]["security"]["capabilities"][1], "overlay");

    let main = window(&config, "main");
    assert_eq!(main["width"], 1280);
    assert_eq!(main["height"], 768);
    assert_eq!(main["backgroundColor"], "#0B0B0C");
    assert_eq!(
        main["decorations"], true,
        "Linux uses native window decorations"
    );
    assert_eq!(main["titleBarStyle"], "Overlay");

    let overlay = window(&config, "overlay");
    assert_eq!(overlay["visible"], false);
    assert_eq!(overlay["width"], 320);
    assert_eq!(overlay["height"], 36);
    assert_eq!(overlay["alwaysOnTop"], true);
    assert_eq!(overlay["transparent"], true);
    assert_eq!(overlay["decorations"], false);
    assert_eq!(overlay["skipTaskbar"], true);
    assert_eq!(overlay["url"], "index.html?window=overlay");

    let icons = config["bundle"]["icon"].as_array().unwrap();
    assert!(
        icons
            .iter()
            .any(|icon| icon.as_str() == Some("icons/icon.png"))
    );
}

#[test]
fn capabilities_are_least_privilege() {
    let main = read_json("capabilities/default.json");
    let overlay = read_json("capabilities/overlay.json");
    assert_eq!(main["identifier"], "main");
    assert_eq!(overlay["identifier"], "overlay");
    assert_eq!(main["windows"][0], "main");
    assert_eq!(overlay["windows"][0], "overlay");

    let main_permissions = main["permissions"].as_array().unwrap();
    let overlay_permissions = overlay["permissions"].as_array().unwrap();
    assert_no_shell(main_permissions);
    assert_no_shell(overlay_permissions);

    let mut strings = Vec::new();
    collect_strings(&main, &mut strings);
    collect_strings(&overlay, &mut strings);
    for text in &strings {
        assert!(
            !text.contains("allow-all"),
            "capabilities must not contain allow-all: {text}"
        );
        assert!(
            !text.contains("$HOME"),
            "filesystem scope must not grant $HOME: {text}"
        );
    }

    let fs_scope = main_permissions
        .iter()
        .find(|permission| permission_id(permission) == "fs:scope")
        .unwrap();
    assert!(fs_scope["allow"].as_array().unwrap().is_empty());

    for permission in overlay_permissions {
        let id = permission_id(permission);
        assert!(
            !id.starts_with("fs:")
                && !id.starts_with("http:")
                && !id.contains("clipboard")
                && !id.starts_with("shell:"),
            "overlay must not receive fs, http, clipboard, or shell: {id}"
        );
    }

    let root = application_data_root();
    assert_eq!(root.file_name().unwrap(), "harbor");
    if let Some(home) = dirs::home_dir() {
        assert_ne!(root, home);
    }
}

#[test]
fn minisign_placeholder_cannot_verify_updates() {
    let text = fs::read_to_string(manifest_dir().join("minisign.pub")).unwrap();
    assert!(text.contains("placeholder"));
    assert!(text.contains("must never authorize an update"));
    for line in text.lines() {
        let line = line.trim();
        assert!(
            !line.to_ascii_lowercase().starts_with("untrusted comment:"),
            "placeholder must not be a minisign public-key file"
        );
        assert!(
            !(line.starts_with("RW")
                && line.len() >= 52
                && line
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')),
            "placeholder must not contain a minisign key row"
        );
    }
}

#[test]
fn window_icon_is_rgba_png() {
    let bytes = fs::read(manifest_dir().join("icons/icon.png")).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert!(bytes.len() >= 26);
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    assert_eq!(width, height);
    assert!(width >= 32);
    assert_eq!(bytes[24], 8, "8-bit samples");
    assert_eq!(bytes[25], 6, "Tauri requires RGBA");
}

#[test]
fn release_panics_abort_and_no_renderer_grant_commands() {
    let workspace = fs::read_to_string(manifest_dir().join("../../../Cargo.toml")).unwrap();
    assert!(workspace.contains("panic = \"abort\""));
    let cargo = fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"harbor\""));
    let build = fs::read_to_string(manifest_dir().join("build.rs")).unwrap();
    assert!(build.contains("\"settings_get\""));
    assert!(build.contains("\"settings_set\""));
    let schema =
        fs::read_to_string(manifest_dir().join("../../../packages/schema/src/commands.ts"))
            .unwrap();
    assert!(schema.contains("settings_get"));
    assert!(!schema.contains("thread_messages"));
}
