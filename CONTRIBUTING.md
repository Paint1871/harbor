# Contributing to Harbor

Thanks for wanting to help. Harbor is a local-first desktop host for coding
agents. The UI, docs, commit messages, and review comments in this repository
are English.

## Before you start

1. Read [CLEANROOM.md](CLEANROOM.md) and certify its statement on every pull
   request: *I did not copy or look at the reference product's source.*
2. Read the product invariants in [AGENTS.md](AGENTS.md). They are not
   optional: no account gate, no paywall, no bundled engines, no proxying of
   vendor API keys.
3. Skim [docs/status.md](docs/status.md) so you do not rebuild something that
   is already listed as unfinished on purpose.

## Development setup

Install stable Rust with rustfmt and Clippy, Node.js 22.11 or newer, pnpm
10.9.0, Git, and ripgrep. On Debian or Ubuntu also install the GTK and
WebKitGTK packages listed in the [README](README.md#requirements).

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm --filter @harbor/desktop tauri dev
```

`pnpm check` is the same four jobs CI runs. Do not open a pull request that
fails it.

## How to send a change

- Keep the pull request independently reviewable. One concern per PR.
- Match the surrounding code. Rust is formatted by rustfmt; TypeScript follows
  the existing desktop and UI packages.
- UI copy is English, written as literals. Do not add an i18n key layer.
- Add tests that exercise the new behaviour. A compile-only check is not
  evidence that a feature works.
- Commit both `Cargo.lock` and `pnpm-lock.yaml` when dependencies change.
- Conventional commits are the house style: `feat(chat): …`, `fix(pty): …`,
  `docs: …`.

Use the pull request template. It includes the clean-room certification.

## GitHub plugin

Forks that enable the GitHub plugin must register their own GitHub App, enable
Device Flow, and set `HARBOR_GITHUB_CLIENT_ID` (or replace `CLIENT_ID` in
`crates/harbor-plugins/src/github.rs`). Permissions: Metadata read; Contents,
Issues, and Pull requests read and write. Harbor never ships a client secret.
Disconnecting deletes the OS keyring item only.

## What we will not merge

- Account gates, credit meters, upgrade screens, or usage paywalls
- Bundled coding engines or vendor API-key proxies
- Reference-product names, logos, screenshots, CDN assets, or marketing copy
- Secrets, signing keys, or a real `minisign.pub` checked in as a placeholder
  replacement without a documented release process
- Silent cloud fallbacks for dictation

## Conduct

[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Be specific in review, and kind.
