# Architecture

Harbor is a Tauri 2 desktop app. The native host is Rust. The renderer is
React 19 on Vite. They talk over a typed IPC contract in `packages/schema`.

## Process model

```text
harbor (Tauri host)
├── renderer (React, no shell of its own)
├── PTY children (allowlisted executables only)
└── ACP children (coding CLIs on PATH, stdio JSON-RPC)
```

The renderer cannot spawn processes. File access is limited to host-granted
roots. Each IPC command is individually allowlisted in
`apps/desktop/src-tauri/permissions/` and `capabilities/`.

## Crates

| Crate | Role |
| --- | --- |
| `apps/desktop/src-tauri` (`harbor-desktop`) | Window, IPC commands, capabilities, crash log |
| `harbor-core` | SQLite, settings, workspaces, agents, threads, layout |
| `harbor-acp` | ACP v1 client, session policy, permission cards |
| `harbor-pty` | PTY spawn behind an executable allowlist |
| `harbor-plugins` | Outbound MCP proxy, OS keyring, GitHub Device Flow |
| `harbor-git` | Diffs for the Chat changes panel |
| `harbor-paths` | App-data and well-known filesystem locations |
| `harbor-speech` | On-device dictation (stubbed in 0.1.0) |
| `harbor-updater` | Signed-update check (placeholder key in 0.1.0) |

## Packages

| Package | Role |
| --- | --- |
| `@harbor/ui` | Design tokens and shared primitives |
| `@harbor/schema` | IPC types shared by host and renderer |
| `@harbor/engine-catalog` | Engine table used for PATH detection |
| `@harbor/desktop` | The desktop renderer |

## Data

Application state lives in a SQLite file under the OS app-data directory
(`harbor.sqlite`). Migrations are in `crates/harbor-core/migrations/`. Session
restore reloads layout only: no processes, no scrollback. Restored terminals
stay paused until the user resumes them.

## Engines

Harbor never bundles a coding engine. Detection reads the login-shell `PATH`
against `packages/engine-catalog`. Chat is ACP v1 over stdio. Code is a PTY.
Some engines need a small ACP adapter package; Harbor can fetch that adapter
on request, still without bundling the engine itself.

## Plugins

The GitHub plugin is Device Flow with no client secret. Access tokens live in
the OS keyring. The outbound MCP proxy is how Harbor talks to GitHub; engine
processes never receive those tokens in their environment.
