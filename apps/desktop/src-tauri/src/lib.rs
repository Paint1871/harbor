//! Native window foundation. Product commands are added in their designated PRs.

pub mod acp_host;
pub mod crash;
pub mod ipc;
pub mod plugins_host;
pub mod pty_host;
pub mod security;

use std::path::PathBuf;

use tauri::Manager;
use tauri_plugin_fs::FsExt;

#[cfg(test)]
mod host_config;

pub fn version() -> &'static str {
    harbor_core::version()
}

/// App-support root `harbor/`. Filesystem plugin scope is this directory, not `$HOME`.
pub fn application_data_root() -> PathBuf {
    dirs::data_dir()
        .expect("could not resolve local application data directory")
        .join("harbor")
}

pub fn run() {
    let data_root = application_data_root();
    crash::install(&data_root);

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("local-navigation")
                .on_navigation(|_, url| security::allows_navigation(url, cfg!(debug_assertions)))
                .build(),
        )
        .setup(move |app| {
            crash::prepare_directory(&data_root)?;
            app.fs_scope().allow_directory(&data_root, true)?;
            #[cfg(windows)]
            if let Some(main) = app.get_webview_window("main") {
                main.set_decorations(false)?;
            }
            // No workspace roots or engine processes are opened on cold start.
            let allowlist = security::ExecutableAllowlist::bootstrap()?;
            acp_host::grant_engines(&allowlist, &harbor_core::engines::recheck());
            app.manage(allowlist);
            app.manage(pty_host::PtyRegistry::default());
            app.manage(acp_host::AcpRegistry::default());
            let db_path = data_root.join("harbor.sqlite");
            let pool = tauri::async_runtime::block_on(harbor_core::db::open(&db_path))?;
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(ipc::handlers())
        .run(tauri::generate_context!())
        .expect("Harbor native runtime failed");
}
