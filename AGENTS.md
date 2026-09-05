# Harbor contribution instructions

Read the supplied `DESIGN.md` before implementation. In the original workspace
it is `../DESIGN.md`, outside this repository. If it is unavailable in a fresh
checkout, obtain the specification from the task owner before implementing
product behavior. It takes precedence over the goal document.

Follow its PR Plan in order, keeping each PR independently reviewable. Implement
only the current PR. PR-01 is workspace bootstrap; PR-02 adds tokens and UI
primitives; PR-03 introduces the Tauri window. Do not pull later features into
earlier PRs. Public 0.1.0 requires PR-01 through PR-27 and their release gates.

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

## Contribution and verification

Read [CLEANROOM.md](CLEANROOM.md) and certify its statement for contributions.
Use original source, copy, and assets. Keep changes scoped to the specified
monorepo directories. Do not add secrets, vendor engines, or reference assets.

Run `pnpm install --frozen-lockfile` and `pnpm check` before submitting. Commit
both lockfiles when dependencies change. Add tests that exercise meaningful
behavior as features land; a scaffold check is not product acceptance evidence.
Chrome changes also require the design's visual QA ritual. Never report a
release gate as passed without its actual build or runtime evidence.
