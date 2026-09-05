fn main() {
    // Register every future custom IPC command here so unlisted windows cannot
    // invoke application commands by bypassing their capability files.
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[])),
    )
    .expect("failed to build Harbor configuration");
}
