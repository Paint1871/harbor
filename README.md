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

To clone this tree and run it locally, start at [Requirements](#requirements)
and [Build and run](#build-and-run). There is no published installer yet.

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
- The updater verifies signed artifacts against the baked release key, but no
  signed release has been published yet, so the install path has never run
  end-to-end.
- Several Settings rows are explanatory copy rather than wired controls.

The standing list is in [docs/status.md](docs/status.md).

## What Harbor will not add

- No account gate, paywall, credit meter, or upgrade screen. The welcome action
  is `Start local`.
- No bundled engines and no proxying of vendor API keys.
- No silent cloud fallback for dictation. On-device is the default; a cloud
  endpoint requires a URL you enter yourself.
- Plugin tokens never enter an engine's environment.

## Requirements

Harbor is a [Tauri 2](https://v2.tauri.app/) desktop app: a Rust host plus a
React 19 renderer. You need both toolchains to compile it. Harbor does **not**
ship coding engines, vendor API keys, or a cloud account. Chat and Code talk to
CLIs you install yourself.

### Operating systems

| OS | Minimum |
| --- | --- |
| macOS | 14 (Sonoma) |
| Windows | 10 x64 |
| Linux | Ubuntu 22.04, or another distro with GTK 3 and WebKitGTK 4.1 |

Unavailable OS features degrade rather than blocking startup. There is no
signed installer yet, so running Harbor means building it from this tree.

### Tools to compile

| Tool | Version | Role |
| --- | --- | --- |
| **Rust** | current **stable**, with `rustfmt` and `clippy` | Native host and crates. [`rust-toolchain.toml`](rust-toolchain.toml) selects the channel and components. The workspace uses Rust edition 2024. |
| **Node.js** | **22.11** or newer | Renderer, Vite, tests, and ACP adapter installs |
| **pnpm** | **10.9.0** (exact; see `packageManager` in `package.json`) | JavaScript workspace |
| **Git** | any recent | Clone the repo; the Chat changes panel also runs `git diff` at runtime |
| **C/C++ toolchain** | platform native | Links Tauri, WebKit/WebView2, and native crates |

`ripgrep` (`rg`) is required only for `pnpm check:brand`, not to launch the app.

Confirm the versions:

```sh
rustc --version
cargo --version
node --version    # v22.11 or newer
pnpm --version    # 10.9.0
git --version
```

### Install the toolchains

Rust via [rustup](https://rustup.rs/). The first `cargo` invocation in this
repo installs the toolchain and components from `rust-toolchain.toml`:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, use the rustup installer and pick the default host
`x86_64-pc-windows-msvc`.

Node.js 22.11+ from [nodejs.org](https://nodejs.org/) or a version manager
(`nvm`, `fnm`, `volta`). Then enable the pinned pnpm:

```sh
corepack enable
corepack prepare pnpm@10.9.0 --activate
```

### Platform libraries

**macOS.** Install the Xcode Command Line Tools (Clang, SDKs, and the linker):

```sh
xcode-select --install
```

**Windows.** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
with the **Desktop development with C++** workload, and the
[WebView2 Evergreen Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
if it is not already present (Windows 11 usually has it).

**Debian / Ubuntu.** Tauri 2 links GTK 3 and the WebKitGTK 4.1 line. Nothing in
the workspace type-checks until these packages are installed:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libxdo-dev \
  libssl-dev build-essential pkg-config
```

Other distros need the same libraries under their own package names. The
optional GitHub plugin stores tokens in the OS keyring (`libsecret` /
GNOME Keyring / KWallet on Linux). If the keyring is locked or missing, Harbor
falls back to a `0600` file under the app-data directory.

### Runtime (optional, but needed for Chat and Code)

The window opens with no engines installed. To actually talk to a coding CLI:

- Install at least one engine from the catalog onto your **login-shell**
  `PATH` (the `PATH` you get in a new terminal, not only the GUI app's
  environment). Harbor probes `$SHELL -l` for that path.
- Log in to that CLI with the vendor's own command (`opencode`, `claude`,
  `codex`, and so on). Harbor never proxies those credentials.
- Some engines (Claude Code, Codex) speak ACP through an adapter package.
  Harbor can fetch that adapter on request via `npm`; it is stored under
  app data, never globally, and never bundled in the repo.
- OpenCode is the engine CI handshakes (`opencode acp`). It is the smallest
  path to a working Chat session.

The GitHub plugin is optional. Forks that enable it must register their own
GitHub App, turn on Device Flow, and set `HARBOR_GITHUB_CLIENT_ID`. Harbor
never ships a client secret.

## Build and run

All commands below are from the **repository root** (the directory that
contains `package.json` and `Cargo.toml`).

1. Install JavaScript workspace dependencies. `--frozen-lockfile` matches CI
   and fails if `pnpm-lock.yaml` is out of date:

   ```sh
   pnpm install --frozen-lockfile
   ```

2. Compile and open the desktop app in development. Vite serves the renderer
   at `http://127.0.0.1:1420`; Tauri wraps it in the native window. The first
   Rust compile downloads crates and can take several minutes:

   ```sh
   pnpm --filter @harbor/desktop tauri dev
   ```

3. On first launch you should see Welcome with no account prompt. Choose
   **Start local**. After that, Harbor opens on the Agent rail. Data lives in
   a local SQLite file; nothing is created in the cloud.

A local unsigned bundle (not a notarized release):

```sh
pnpm --filter @harbor/desktop tauri build
```

On macOS the `.app` lands under `target/release/bundle/macos/`. Other
platforms write their artifacts next to that under `target/release/bundle/`.
Self-built binaries verify updates against the release public key baked into
this tree, so they accept only artifacts signed with the matching secret key —
which no public release has been signed with yet.

Work on the design system without the native host:

```sh
pnpm --filter @harbor/ui dev
```

That preview is a component fixture. It never talks to an engine.

### After it is running

Install a coding CLI, confirm `command -v <binary>` works in a new terminal,
then use Recheck in Harbor (or restart) so detection picks it up. Chat uses
ACP v1 over stdio; Code opens a real PTY. Without a detected engine, both
modes still render — they just have nobody to talk to.

### Where data goes

Display name `Harbor`, directory name `harbor`:

| OS | App-data root |
| --- | --- |
| macOS | `~/Library/Application Support/harbor/` |
| Windows | `%APPDATA%\harbor\` |
| Linux | `$XDG_DATA_HOME/harbor/` (usually `~/.local/share/harbor/`) |

The database is `harbor.sqlite` in that directory. Crash logs go to
`logs/crash.log`. ACP adapters Harbor installs live in `adapters/`. Optional
vendor logos from `scripts/fetch-engine-logos.sh` go in `engine-icons/`.

### If the build fails

- **`pnpm` is the wrong version.** The workspace pins **10.9.0**. Run
  `corepack prepare pnpm@10.9.0 --activate` rather than a global `npm i -g pnpm`.
- **Linux: missing WebKitGTK.** `cargo` errors about `webkit2gtk-4.1` or
  `libgtk-3` mean the packages in [Platform libraries](#platform-libraries)
  are not installed. A bare `cargo test` of this workspace needs them too.
- **Windows: `link.exe` not found.** The MSVC C++ workload is missing.
- **Engines not detected.** The CLI is on a GUI-invisible `PATH`. Install it
  where your login shell can see it (`~/.local/bin`, Homebrew, nvm, …) and
  restart Harbor.
- **`pnpm install` wants to change the lockfile.** Use
  `--frozen-lockfile` for a source checkout. Only drop that flag when you
  intend to change dependencies.

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
