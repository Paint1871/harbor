# Status

Harbor 0.1.0 is a development target, not a published release. This page lists
what is unfinished so contributors do not have to rediscover it.

Last reviewed 2026-09-21.

## Release gates that have not been run

- `.github/workflows/release.yml` builds macOS/Windows/Ubuntu bundles, signs
  every artifact and `manifest.json` with the release key, and publishes a
  GitHub release — but the workflow has never run, so none of that is proven
  yet. macOS code-signing/notarization additionally needs the `APPLE_*`
  secrets; without them the bundle is unsigned for Gatekeeper and must not
  ship to end users.
- `apps/desktop/src-tauri/minisign.pub` holds the real release public key and
  `updater_install` downloads, verifies, and stages a signed artifact against
  the signed manifest — but no signed release has been published yet, so the
  update path has never run end-to-end in the wild. The signing and
  publishing steps are written down in [releasing.md](releasing.md); the
  secret key's custody is the open operational question there, not a code
  one.
- No desktop end-to-end run on Windows or Linux exists; the ACP handshake and
  Windows PTY workflows are the only cross-platform gates that actually run
  in CI.

## Known gaps

- **Dictation.** `dictation_begin` / `dictation_end` are unimplemented.
  `harbor_speech::engine_available()` is `false`. There is no silent cloud
  fallback; that is intentional. The settings page states this instead of
  offering buttons that would fail.
- **Settings.** Recheck engines is a live host action. UI zoom, startup mode,
  default shell (including PowerShell/cmd), notification sound, and per-kind
  notification filters honor their stored values. Desktop notifications fire
  as real OS toasts via `tauri-plugin-notification` when the window is not
  focused — the in-app bell still carries the event either way.
- **Voice.** Dictation and a title-bar mute orb are not shipped. Welcome has a
  decorative orbit; hold-to-talk is a no-op.
- **Faces.** `face_preview` returns an SVG data URL. New agents get a stable
  atlas slot from a hash of the agent id unless the builder picks a face.
- **Plugins.** Only the GitHub connection is real. Its primary path is the
  GitHub App device flow (`plugin_connect`), which needs a client id from
  Settings → General or `HARBOR_GITHUB_CLIENT_ID`; pasting a PAT remains as
  the explicit fallback. Every other catalog row renders as "soon". The MCP
  sidecar loads the granted keyring credential itself and serves five GitHub
  tools (`github_me`, repos, issues, PR list/read) — tokens never reach the
  engine environment or tool output. An agent without a grant sees the tools,
  calls one, and the call becomes a pending approval row plus a "permission"
  inbox event; approving it on the Plugins page flips the same
  `plugin_grants` row the grant checkboxes toggle, and the running session
  picks it up — and loses a revoked grant — on the next call without a
  respawn. There are no write tools yet, so the approval card's "grant"
  action is the only action this build can execute.
- **Code-mode panes.** The thread pane is a doorway into Chat mode, not a
  second chat surface, and the browser pane hands URLs to the system browser;
  an embedded (wry child webview) preview is post-0.1.0 and the app CSP forbids
  frames regardless.
- **ACP extras.** Agent `home_path` is stored on create when the builder
  picks a folder; otherwise the ACP cwd still falls back to a granted place or a
  temp directory. Live ACP spawns attach a `harbor-plugins` stdio MCP sidecar;
  plugin tokens stay in that process and never enter the engine environment.

## What is built

Agent, Code, and Chat modes run against real local state. Terminals are PTYs.
Chat and Agent conversations use ACP v1. Workspaces, threads, agents, memory,
layout, and notifications persist in SQLite; full-text search covers both
agent chats and workspace threads. A reply landing in a folder thread nobody
is watching marks it `unread`, which drives the thread list filter and the
dashboard's "waiting on you" count across all workspaces; opening the thread
marks it read again. GitHub Device Flow stores tokens in the
OS keyring. ACP adapters install under the app-data directory only, pinned to
an exact version and verified against the published `dist.integrity` hash
before they run. Scheduled routines (weekdays/weekly) fire while the app is
open: each due routine stamps itself, creates a chat with its brief staged as
`pending_agent_prompt:<chatId>`, and records a filtered inbox event;
`manual` stays "Run now" only.

See the [README](../README.md) for how to build and run, and
[architecture.md](architecture.md) for crate boundaries.
