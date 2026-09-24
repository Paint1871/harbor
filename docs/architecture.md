# Architecture

Harbor is a Tauri 2 desktop app. The native host is Rust. The renderer is
React 19 on Vite. They talk over a typed IPC contract in `packages/schema`.

## Process model

```text
harbor (Tauri host)
├── renderer (React, no shell of its own)
├── PTY children (allowlisted executables only)
└── ACP children (coding CLIs on PATH, stdio JSON-RPC)
    └── harbor mcp-plugins (stdio MCP sidecar, spawned by the engine)
```

The renderer cannot spawn processes. File access is limited to host-granted
roots, and those roots only ever come from the native picker: the host keeps
the picked path and returns an opaque, single-use pick id that exactly one
command can redeem — a renderer-supplied path can never create a grant. Each
IPC command is individually allowlisted in
`apps/desktop/src-tauri/permissions/` and `capabilities/`; scope entries are
revoked when the last workspace, place, or agent home that referenced them is
removed.

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
| `harbor-updater` | Signed-update check (real baked key; no signed release published yet) |

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
on request, still without bundling the engine itself. Catalog adapters pin an
exact version plus the published `dist.integrity` hash, and the install
verifies npm's lockfile receipt against both before the adapter can run.

## Plugins

The GitHub plugin is Device Flow with no client secret. Access tokens live in
the OS keyring. The outbound MCP proxy is how Harbor talks to GitHub; engine
processes never receive those tokens in their environment.

Grants are per-agent rows, not implied by a connection. The sidecar gets the
granted plugin ids, the agent id, and the database path in its environment —
never a token — and re-checks `plugin_grants` per call, so a grant toggled
under Plugins mid-session — on or off — takes effect without a respawn. A call without a grant raises a
`plugin_approvals` row and a `permission` inbox event; resolving it on the
Plugins page runs `set_agent_grant`, which is the same switch as the grant
checkboxes.
