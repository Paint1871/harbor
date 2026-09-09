# Status

Harbor 0.1.0 is a development target, not a published release. This page lists
what is unfinished so contributors do not have to rediscover it.

Last reviewed 2026-09-09.

## Release gates that have not been run

- No signed or notarized macOS build
- No Windows preview installer (the Windows PTY workflow is the gate for that)
- `apps/desktop/src-tauri/minisign.pub` is a placeholder, so `updater_check`
  never reports an available update and install refuses

## Known gaps

- **Dictation.** `dictation_begin` / `dictation_end` are unimplemented.
  `harbor_speech::engine_available()` is `false`. There is no silent cloud
  fallback; that is intentional.
- **Linux title bar.** `tauri.conf.json` sets `titleBarStyle: Overlay` on every
  platform. Linux should use native decorations plus an in-content toolbar.
- **Settings.** Several rows (zoom, startup mode, shell, Recheck, notification
  filters) are still explanatory copy rather than wired controls.
- **Voice.** Dictation and a title-bar mute orb are not shipped. Welcome has a
  decorative orbit; hold-to-talk is a no-op.
- **Faces.** The face slot is `faceIndex % 64`, not a stable hash of the agent
  id. `face_preview` is typed as `{ pngB64 }` while core returns an SVG data URL.
- **Fonts.** Inter is referenced via `local()` only; no webfont is bundled.
- **ACP extras.** `mcp_servers` is always empty in `SpawnSpec` so plugin tokens
  never reach an engine. Agent `home_path` is written as `""` on create; the
  ACP cwd falls back to a granted place or a temp directory.

## What is built

Agent, Code, and Chat modes run against real local state. Terminals are PTYs.
Chat and Agent conversations use ACP v1. Workspaces, threads, agents, memory,
layout, and notifications persist in SQLite. GitHub Device Flow stores tokens
in the OS keyring.

See the [README](../README.md) for how to build and run, and
[architecture.md](architecture.md) for crate boundaries.
