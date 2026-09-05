# Contributing

Read `AGENTS.md` and `CLEANROOM.md`. Implement against the supplied design.

Forks that enable the GitHub plugin must register their own GitHub App, enable
Device Flow, and set `HARBOR_GITHUB_CLIENT_ID` (or replace `CLIENT_ID` in
`crates/harbor-plugins/src/github.rs`). Permissions: Metadata read; Contents,
Issues, and Pull requests read and write. Harbor never ships a client secret.
Revoke deletes the OS keyring item only.

Run `pnpm install --frozen-lockfile` and `pnpm check` before opening a pull request.
