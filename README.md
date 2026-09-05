# Harbor

an open desktop host for coding agents

Harbor is a free, Apache-2.0 desktop host for coding CLIs already installed on
your PATH. It is local-first, requires no account, and has no paywall. Engines
are not bundled: they use your existing vendor accounts and bill you directly.
Harbor never proxies their API keys.

## Development status

PR-01 established the Rust and pnpm/Turborepo workspaces and CI checks. PR-02
adds `@harbor/ui`: Black/Light tokens, theme and motion handling, and six UI
primitives. PR-03 adds the native Harbor window (1280×768, `#0B0B0C` before
first paint), a hidden overlay stub, least-privilege capabilities, a host-owned
executable allowlist, and a panic hook that writes `logs/crash.log`. Version
0.1.0 is a development target, not a published release.

Run the isolated component preview with `pnpm --filter @harbor/ui dev`. See
[the UI package](packages/ui/README.md) for component contracts and verification.

The planned app has three modes: named teammates in Agent, real PTY terminals
and files in Code, and folder-scoped ACP v1 threads with diffs in Chat. The
primary welcome action will be **Start local**.

The specified stack is Tauri 2, Rust, React 19, TypeScript, Vite, SQLite/sqlx,
xterm.js, and CodeMirror 6. Minimum OS targets are macOS 14, Windows 10 x64,
and Ubuntu 22.04. The first public release targets signed, notarized macOS;
Windows preview depends on the Windows PTY CI gate. Linux packages follow later.

## Bootstrap checks

Install stable Rust with rustfmt and Clippy, Node.js 22.11 or newer, pnpm 10.9.0,
Git, and ripgrep. The Rust toolchain file selects the required components.

From this repository root:

```sh
pnpm install --frozen-lockfile
pnpm check
```

Individual checks:

```sh
bash scripts/deny-brand.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
pnpm check:workspace
```

Turborepo runs the desktop package's Cargo check and the UI package's TypeScript
check and Vite preview build. Caching is disabled for that
task because Rust sources and the lockfile span package boundaries; Cargo still
handles incremental compilation. No remote cache account is required.

## Layout

```text
apps/desktop/            Desktop package and native host bootstrap
crates/harbor-core/      Shared local application core
packages/ui/            Design tokens, primitives, and development preview
scripts/                Repository checks
.github/workflows/      Continuous integration
```

The engine catalog, IPC schema, and additional Rust crates will be added
in their designated PRs. The implementation specification is supplied separately
as `../DESIGN.md` in this workspace; read its PR Plan before contributing.

See [AGENTS.md](AGENTS.md) for contribution instructions,
[CLEANROOM.md](CLEANROOM.md) for provenance rules, and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.
The [Apache-2.0 license](LICENSE) covers Harbor's source.
