# Harbor verification pass — 2026-09-07 (third pass)

Read-only audit of the live `harbor/` tree against `DESIGN.md` Harbor 0.1.0 DoD,
followed by a cleanup pass. This file overwrites the 2026-09-06 report, which had
gone stale: four of its five blockers were fixed by the work that landed after it.

**Certification:** I did not copy or look at the reference product's source.

## 1. Verdict

**mixed, but honest again.**

The second pass's blockers 1–4 are resolved in the tree: `layout_restore` /
`workspace_ensure_tab` / `pane_create` are wired end to end and covered by
`CodeMode.test.tsx` and `layout::tests::opening_a_folder_creates_a_tab_that_restore_can_load`;
`session_search` and `mail_send` are reachable from the Agent gear panel and page;
plugin grants and approvals are live in `GearPanel` and the Plugins destination.

This pass found a different class of problem — the chrome restyle in `49e5d0e`
had traded honesty for visual fidelity. That is fixed (§3). Blocker 5 (Linux
title bar) stands, and the release gates remain unverified.

## 2. Commands run — 2026-09-07

All from `harbor/`.

| Command | Result |
| --- | --- |
| `bash scripts/deny-brand.sh` | `deny-brand: clean`, exit 0 |
| `pnpm check:workspace` | 4 packages, exit 0 |
| `pnpm check:test` | 12 files, 37 tests passed |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | harbor-core 28, harbor-desktop 16, harbor-acp 12+2, others green |
| `.local/verify/chrome-check.mjs` | titlebar 1, modes 3, no page errors, no paywall copy |

No signed, notarized, or Windows build was produced. No release gate is claimed.

## 3. Fixed in this pass

The `49e5d0e` restyle reproduced the reference product's chrome literally,
including things Harbor exists to not have.

| Problem | Fix |
| --- | --- |
| `Footer` showed a `Notch / Credits 9,684` stat block and a `PRO` plan badge; `Dashboard` showed a `Credits` tile reading `PRO · resets in 12d`. Direct violation of the GOAL.md hard rule and of Appendix A ("What are credits? Harbor has none."). | The stat block is gone entirely — both rows were subscription-shell chrome. The footer is the local profile plus a muted `Free · local`, and the credit tile is removed. |
| `ui-structure.test.tsx` asserted `expect(shell).toContain("Credits")` — the guard had been inverted to lock the violation in. | Asserts no `Credits`, no `PRO`, no `Upgrade`, plus the orb seat and the Workspace menu. |
| The mute orb was deleted from the title bar (`OrbSeat.tsx` removed). 0.1.0 DoD item 8 and K16. | `OrbSeat` restored: 28px seat, click states `Voice is off until a later release` and links to Settings → Voice. Hold stays a no-op. |
| `WorkspaceMenu.tsx` and its CSS were dead code; the title bar had a bare `Tidy` button in code mode only. K30 order was wrong (sidebar toggle sat next to the wordmark). | Title bar is now wordmark, mode switch, flex, `Workspace ▾`, orb, bell, sidebar toggle. |
| `Dashboard` reported invented metrics (`Agents 4`, `Turns today 128 / 31.6k tokens`) and four fabricated activity rows naming agents that do not exist. 0.1.0 has no usage meter. | Counts `agent_list`, `workspace_list` and `thread_list` rows, lists real threads, and shows an empty state otherwise. |
| `Inbox` rendered five hardcoded events including `Local agent failed`. | Reads the `notifications` table via new `notifications_list` / `notifications_mark_read` commands and shows an empty state when there is nothing. |
| `Destinations` carried a permanent hardcoded `1` badge on Dashboard. | Removed. |
| CI ran neither `cargo test` nor the renderer suite, so 34 existing tests never gated a push. | `ci.yml` runs `cargo test --workspace --locked` and `pnpm check:test`; `turbo.json` gained a `test` task. |
| **CI had never passed.** Every push since PR-03 failed the Rust job: `ubuntu-22.04` has no GTK/WebKitGTK headers, so `harbor-desktop` could not compile and Clippy exited 101. | `ci.yml` installs the Tauri system libraries (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, and friends) before any cargo step. |
| **Windows PTY had never passed.** `harbor-pty`'s `refuses_relative_and_unlisted_programs` used Unix fixtures; `Path::new("/bin/bash").is_absolute()` is `false` on Windows, so the case that should be `Denied` returned `Relative`. | The fixtures are per-platform. The assertions are unchanged, so the gate still tests the same rule. |
| `.local/verify/chrome-check.mjs` had been failing since the restyle: it never clicked `Start local`, so it timed out on `.harbor-titlebar`. | Clicks through Welcome, scopes the orb notice, and now also fails on paywall copy anywhere in the shell. |

## 4. Open items

### Blocker for the 0.1.0 DoD

1. **Linux title bar.** `tauri.conf.json` sets `titleBarStyle: Overlay` for every
   platform and `host_config.rs` asserts it. The design wants native decorations
   plus an in-content toolbar on Linux.

### Debt, allowed or post-0.1.0

- `dictation_begin` / `dictation_end` are unimplemented and `harbor_speech::engine_available()`
  is `false`. Allowed; there is no silent cloud fallback.
- The updater cannot authorize an install while `minisign.pub` is a placeholder.
  `updater_check` never reports `available: true`. Allowed and honest.
- `mcp_servers` is always empty in `SpawnSpec`. Allowed; plugin tokens never
  reach an engine.
- Settings pages 2–5 are still largely explanatory copy (zoom, startup mode,
  shell, Recheck, notification filters are not wired).
- Rail still carries Dashboard/Routines/Plugins/Skills rows at the top. Changelog
  v0.1.20 moved destinations to the footer and command bar.
- Face slot is `faceIndex % 64`, not `blake3(agent_uuid) % 64`. Schema types
  `face_preview` as `{ pngB64 }` while core returns an SVG data URL string.
- Inter is referenced via `local()` only; no bundled webfont.
- The orb is CSS, not the specified plasma/orbit layers. Allowed as 0.1.0 mute chrome.
- No signed or notarized macOS build, and the Windows PTY CI gate was not run.
- `home_path` is written as `""` on agent create; the ACP cwd falls back to a
  granted place or a temp directory.

## 5. Reference comparison

The reference product's marketing site returns HTTP 403 to automated fetches,
as `DESIGN.md` already records. No live CSS, HTML, or asset was retrieved in this pass, and none is in
the repo. The comparison above is against `DESIGN.md` and `GOAL.md` only.

## Contribution certification

I did not copy or look at the reference product's source. This contribution is
original code and copy written for Harbor under Apache-2.0. No reference-product
assets or credentials were added. No asset provenance was altered or invented.
