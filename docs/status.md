# Status

Harbor 0.1.0 is a development target, not a published release. This page lists
what is unfinished so contributors do not have to rediscover it.

Last reviewed 2026-09-15.

## Release gates that have not been run

- No signed or notarized macOS build
- No Windows preview installer (the Windows PTY workflow is the gate for that)
- `apps/desktop/src-tauri/minisign.pub` is a placeholder, so `updater_check`
  never reports an available update and install refuses

## Known gaps

- **Dictation.** `dictation_begin` / `dictation_end` are unimplemented.
  `harbor_speech::engine_available()` is `false`. There is no silent cloud
  fallback; that is intentional.
- **Settings.** Startup mode and Recheck still need tighter wiring as live
  chrome controls. UI zoom, default shell, desktop notifications, notification
  sound, and per-kind notification filters honor their stored values.
- **Voice.** Dictation and a title-bar mute orb are not shipped. Welcome has a
  decorative orbit; hold-to-talk is a no-op.
- **Faces.** `face_preview` returns an SVG data URL. New agents get a stable
  atlas slot from a hash of the agent id unless the builder picks a face.
- **ACP extras.** Agent `home_path` is stored on create when the builder
  picks a folder; otherwise the ACP cwd still falls back to a granted place or a
  temp directory. Live ACP spawns attach a `harbor-plugins` stdio MCP sidecar;
  plugin tokens stay in that process and never enter the engine environment.

## What is built

Agent, Code, and Chat modes run against real local state. Terminals are PTYs.
Chat and Agent conversations use ACP v1. Workspaces, threads, agents, memory,
layout, and notifications persist in SQLite. GitHub Device Flow stores tokens
in the OS keyring.

See the [README](../README.md) for how to build and run, and
[architecture.md](architecture.md) for crate boundaries.
