# Security policy

Harbor is local-first. Please do not file a public issue for a vulnerability
that could put a user's files, tokens, or processes at risk.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting on this repository
(**Security → Report a vulnerability**), or contact a maintainer through the
method on their public GitHub profile.

Include:

- The affected commit or version
- The impact
- A reproduction if you have one

You should hear back within a week. Please give us a reasonable window to ship
a fix before any public disclosure.

## Scope

**In scope:** the Harbor desktop host, its IPC surface, the PTY executable
allowlist, plugin token storage, ACP session handling, and update verification.

**Out of scope:** engines Harbor hosts (Claude Code, Codex, OpenCode, and the
rest of the catalog). Report those to their vendors. Harbor does not bundle
them and does not proxy their API keys.

## Secrets and signing

Never commit `.env` files, API keys, GitHub App client secrets, or code-signing
material. `HARBOR_GITHUB_CLIENT_ID` is a public Device Flow client id; Harbor
never ships a client secret.

`apps/desktop/src-tauri/minisign.pub` is a placeholder. The updater will not
authorize an install until a real release key is documented and substituted.
