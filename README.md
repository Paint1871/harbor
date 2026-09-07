# Harbor

**an open desktop host for coding agents**

Harbor is a free, Apache-2.0 desktop application that hosts the coding CLIs you
already have installed. It is local-first, needs no account, and has no paywall,
credit meter, or plan tier.

Harbor is an open-source alternative to the closed, subscription-gated desktop
hosts in this category. Those products put a monthly fee and a credit balance in
front of an application whose core job is launching third-party CLIs from your
`PATH` — CLIs that already bill through your own vendor accounts. Harbor takes
the same idea and gives it away: the source is public, the data is a SQLite file
on your disk, and your engine credentials never pass through anyone's server.

Harbor does not replace Claude Code, Codex, Cursor, or OpenCode. It **hosts**
them. Engines are never bundled and Harbor never proxies their API keys.

## Three modes in one window

| Mode | What it holds |
| --- | --- |
| **Agent** | Named teammates with a brief, memory, granted places, generated faces, and their own chats |
| **Code** | Real PTY terminals over your folders, a CodeMirror file pane, a browser pane, and a thread pane |
| **Chat** | Folder-scoped threads over ACP v1, with permission cards and a changes panel |

## What is actually built

Every mode above runs against real local state. Terminals are real PTYs. Chat
and Agent conversations go over ACP v1 to a CLI on your `PATH`. Workspaces,
threads, agents, memory, layout, and notifications live in a local SQLite
database. The GitHub plugin uses Device Flow with no client secret in the
binary, and tokens go to the OS keyring rather than into any engine's
environment.

**Harbor 0.1.0 is a development target, not a published release.** Being honest
about the gaps:

- Dictation `begin`/`end` are unimplemented and no speech engine ships yet.
- The updater cannot authorize an install: `minisign.pub` is a placeholder, so
  the update check always reports "not available" and install refuses.
- Several Settings rows are explanatory copy rather than wired controls.
- On Linux the window uses an overlay title bar instead of native decorations.
- No signed or notarized build has been produced, on any platform.

`docs-src/verify-pass.md` is the standing audit and lists open items in full.

## What Harbor will not add

- No account gate, paywall, credit meter, or upgrade screen. The welcome action
  is `Start local`.
- No bundled engines and no proxying of vendor API keys.
- No silent cloud fallback for dictation. On-device is the default; a cloud
  endpoint requires a URL you enter yourself.
- Plugin tokens never enter an engine's environment.

## Requirements

Stable Rust with rustfmt and Clippy, Node.js 22.11 or newer, pnpm 10.9.0, Git,
and ripgrep. The `rust-toolchain.toml` file selects the required components.

Minimum operating systems are macOS 14, Windows 10 x64, and Ubuntu 22.04.

On Debian or Ubuntu, Tauri needs GTK and WebKitGTK development packages before
anything will compile:

```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libxdo-dev \
  libssl-dev build-essential pkg-config
```

## Building and running

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm check
```

To run the desktop application in development:

```sh
pnpm --filter @harbor/desktop tauri dev
```

To work on the design system in isolation:

```sh
pnpm --filter @harbor/ui dev
```

## Checks

`pnpm check` runs everything CI runs, in the same order:

| Command | What it guards |
| --- | --- |
| `pnpm check:brand` | Clean-room guard over tracked and untracked files |
| `pnpm check:workspace` | TypeScript and Vite builds for every package |
| `pnpm check:test` | Renderer test suite |
| `pnpm check:rust` | `cargo fmt`, Clippy with `-D warnings`, and the Rust tests |

Three GitHub Actions workflows run on every push and pull request: **CI** (the
four checks above), **ACP handshake** (a live handshake against OpenCode), and
**Windows PTY** (the PowerShell PTY gates, which decide whether a Windows
preview build is allowed at all).

## Layout

```text
apps/desktop/            Desktop package, native host, and renderer
crates/harbor-core/      Local application core: data model and commands
crates/harbor-acp/       ACP v1 client and session policy
crates/harbor-pty/       PTY spawning behind an executable allowlist
crates/harbor-plugins/   Outbound MCP proxy, keyring, GitHub Device Flow
packages/ui/             Design tokens, primitives, and a component preview
packages/schema/         The IPC contract shared by host and renderer
scripts/                 Repository checks
```

## Security model

Process creation stays in Rust behind an executable allowlist; the renderer has
no shell command of its own. File access is restricted to host-granted roots.
The renderer capability list is least-privilege and each IPC command is
individually allowed. Session restore brings back layout only — never processes
or scrollback — and restored terminals stay paused until you resume them.

## Contributing

Read [AGENTS.md](AGENTS.md) for contribution instructions and
[CLEANROOM.md](CLEANROOM.md) for provenance rules, which every contribution must
certify. [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) covers community expectations.

Run `pnpm check` before submitting, and commit both lockfiles when dependencies
change. Add tests that exercise real behaviour; a scaffold check is not
acceptance evidence for a feature.

## License

[Apache-2.0](LICENSE). Harbor's logo, agent faces, orb, and engine marks are
original artwork. Engine marks are Harbor's own drawings and are deliberately
not the vendors' trademarks.
