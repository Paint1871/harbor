<p align="center">
  <img src="packages/ui/src/harbor-logo.png" alt="Harbor" width="88" height="88">
</p>

<h1 align="center">Harbor</h1>

<p align="center"><strong>an open desktop host for coding agents</strong></p>

<p align="center">
  <a href="https://github.com/Paint1871/harbor/actions/workflows/ci.yml"><img src="https://github.com/Paint1871/harbor/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="Apache-2.0"></a>
</p>

Harbor is a free, [Apache-2.0](LICENSE) desktop application that hosts the coding
CLIs you already have installed. It is local-first, needs no account, and has no
paywall, credit meter, or plan tier.

Harbor does not replace Claude Code, Codex, Cursor, or OpenCode. It **hosts**
them. Engines are never bundled, and Harbor never proxies their API keys.

Closed hosts in this category put a monthly fee in front of an app whose core
job is launching third-party CLIs from your `PATH` — CLIs that already bill
through your own vendor accounts. Harbor gives that idea away: the source is
public, the data is a SQLite file on your disk, and your engine credentials
never pass through anyone's server.

## Three modes in one window

| Mode | What it holds |
| --- | --- |
| **Agent** | Named teammates with a brief, memory, granted places, generated faces, and their own chats |
| **Code** | Real PTY terminals over your folders, a CodeMirror file pane, a browser pane, and a thread pane |
| **Chat** | Folder-scoped threads over ACP v1, with permission cards and a changes panel |

## Hosted engines

Harbor detects CLIs on your login-shell `PATH`. Chat talks ACP v1 over stdio.
Code is a real PTY. The catalog currently knows:

OpenCode, Claude Code, Codex, Cursor, Grok, Gemini CLI, GitHub Copilot, Droid,
Factory, Kimi Code, Muse Code, Amp, Antigravity, and Aider.

Harbor draws its own mark for every engine, so two terminals running different
CLIs never look alike. Those marks are original Harbor artwork, not the vendors'
trademarks.

## Status

**Harbor 0.1.0 is a development target, not a published release.** There is no
signed or notarized build yet.

What is actually built: terminals are real PTYs. Chat and Agent conversations go
over ACP v1 to a CLI on your `PATH`. Workspaces, threads, agents, memory, layout,
and notifications live in a local SQLite database. The optional GitHub plugin
uses Device Flow with no client secret in the binary; tokens go to the OS
keyring rather than into any engine's environment.

Known gaps, in short:

- Dictation `begin` / `end` are unimplemented and no speech engine ships yet.
- The updater cannot authorize an install: `minisign.pub` is a placeholder.
- Several Settings rows are explanatory copy rather than wired controls.
- On Linux the window uses an overlay title bar instead of native decorations.

The standing list is in [docs/status.md](docs/status.md).

## What Harbor will not add

- No account gate, paywall, credit meter, or upgrade screen. The welcome action
  is `Start local`.
- No bundled engines and no proxying of vendor API keys.
- No silent cloud fallback for dictation. On-device is the default; a cloud
  endpoint requires a URL you enter yourself.
- Plugin tokens never enter an engine's environment.

## Requirements

- Stable Rust with rustfmt and Clippy (`rust-toolchain.toml` selects these)
- Node.js 22.11 or newer
- pnpm 10.9.0
- Git and ripgrep

Minimum operating systems are macOS 14, Windows 10 x64, and Ubuntu 22.04.

On Debian or Ubuntu, Tauri needs GTK and WebKitGTK development packages before
anything will compile:

```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libxdo-dev \
  libssl-dev build-essential pkg-config
```

## Build and run

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm check
```

Run the desktop application in development:

```sh
pnpm --filter @harbor/desktop tauri dev
```

Work on the design system in isolation:

```sh
pnpm --filter @harbor/ui dev
```

Install at least one coding CLI and make sure it is on your login-shell `PATH`
before expecting Chat or Code to talk to an engine.

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

## Engine logos

If you would rather see the real vendor logos, `scripts/fetch-engine-logos.sh`
downloads each vendor's own artwork from that vendor's own domain into a runtime
directory outside the repository:

```sh
bash scripts/fetch-engine-logos.sh
```

Harbor reads `<app data>/engine-icons/<engine-id>.{svg,png,webp}` on start and
falls back to its own mark for anything missing. The files are never committed
and never ship with Harbor. Whether your use of a vendor's mark is permitted is
between you and that vendor's brand guidelines. Delete the directory to go back
to the drawn marks.

## Layout

```text
apps/desktop/            Desktop package, native host, and renderer
crates/harbor-core/      Local application core: data model and commands
crates/harbor-acp/       ACP v1 client and session policy
crates/harbor-pty/       PTY spawning behind an executable allowlist
crates/harbor-plugins/   Outbound MCP proxy, keyring, GitHub Device Flow
packages/ui/             Design tokens, primitives, and a component preview
packages/schema/         The IPC contract shared by host and renderer
packages/engine-catalog/ Engine table used for PATH detection
docs/                    Contributor-facing status and architecture
scripts/                 Repository checks
```

A slightly fuller map lives in [docs/architecture.md](docs/architecture.md).

## Security model

Process creation stays in Rust behind an executable allowlist; the renderer has
no shell command of its own. File access is restricted to host-granted roots.
The renderer capability list is least-privilege and each IPC command is
individually allowed. Session restore brings back layout only — never processes
or scrollback — and restored terminals stay paused until you resume them.

Please report vulnerabilities privately. See [SECURITY.md](SECURITY.md).

## Contributing

Harbor is English-only in the UI and in this repository.

Read [CONTRIBUTING.md](CONTRIBUTING.md) for how to build, test, and open a pull
request, [AGENTS.md](AGENTS.md) for product invariants coding agents should
follow, and [CLEANROOM.md](CLEANROOM.md) for provenance rules, which every
contribution must certify. [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) covers
community expectations.

Run `pnpm check` before submitting, and commit both lockfiles when dependencies
change. Add tests that exercise real behaviour; a scaffold check is not
acceptance evidence for a feature.

## License

[Apache-2.0](LICENSE). Harbor's logo, agent faces, orb, and engine marks are
original artwork. Engine marks are Harbor's own drawings and are deliberately
not the vendors' trademarks.
