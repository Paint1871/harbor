# Harbor contribution instructions

This is a living codebase. Implement against the product invariants below, not
against an external specification that is not in this repository. The UI, docs,
and commit messages are English.

## Product invariants

- Name: Harbor; binary: `harbor`; bundle: `app.harbor.desktop`.
- Tagline: `an open desktop host for coding agents`.
- English UI literals, no translation-key layer.
- Local-first, no account gate. The welcome primary action is `Start local`.
- Host installed engines; do not bundle them or proxy vendor credentials.
- Tauri 2 + Rust + React 19 + TypeScript + Vite, SQLite/sqlx, xterm.js, CodeMirror 6.
- Chat uses folder threads and ACP v1. Code contains Terminal and Files panes.
- Plugin tokens stay in the OS keyring and never enter engine environments.
- Restore layout, not processes or scrollback; restored terminals stay paused.
- Dictation defaults to on-device; cloud requires an explicit user URL.
- macOS 14, Windows 10 x64, Ubuntu 22.04 minimums. Degrade unavailable OS features.

## Working in this tree

Keep changes scoped to the concern at hand. Do not add features "while you are
there." Do not invent protocol, OAuth, paywalls, or cloud backends that the
invariants forbid.

Match existing English UI copy (`Start local`, `Ask anything...`, and the rest
of the chrome). If a string is unspecified, write a short literal in the same
voice; do not add an i18n framework.

The webview reaches the host only through `call()` in `apps/desktop/src/ipc.ts`,
typed by `HarborCommands` in `packages/schema/src/commands.ts`. A new command
lands in four places: the `#[tauri::command]` function, `generate_handler!`,
`build.rs` plus `capabilities/default.json`, and that contract. Two tests in
`ipc.rs` fail if you miss one.

Architecture and crate boundaries are in [docs/architecture.md](docs/architecture.md).
Known gaps are in [docs/status.md](docs/status.md). Do not report a listed gap
as done without the actual build or runtime evidence.

## Contribution and verification

Read [CLEANROOM.md](CLEANROOM.md) and certify its statement for contributions.
Use original source, copy, and assets. Do not add secrets, vendor engines, or
reference-product assets.

Run `pnpm install --frozen-lockfile` and `pnpm check` before submitting. Commit
both lockfiles when dependencies change. Add tests that exercise meaningful
behavior as features land; a scaffold check is not product acceptance evidence.
Never report a release gate as passed without its actual build or runtime
evidence.
