# Harbor verification pass — 2026-09-06 (second pass)

Independent read-only audit of the live `harbor/` tree against `DESIGN.md` Harbor 0.1.0 DoD. Commands were re-run after implementation work landed. No product code was changed. This file overwrites the stale first-pass report.

**Certification:** I did not copy or look at the reference product's source.

## 1. Verdict

**mixed**

Pass-1 fakes are gone. Agent chats, gear memory/places, permission cards, Chat/Agent ACP send, appearance persist, face preview / draft-with-AI, and layout *commands* are real. `deny-brand`, `harbor-core` / `harbor-desktop` / `harbor-acp` tests, clippy `-D warnings`, desktop vitest, and `@harbor/ui check` all exited 0.

That is not product acceptance for Harbor 0.1.0. Session restore is not end-to-end (no `workspace_tabs` rows, renderer never loads `layout_json`, `pty_pause` is a host no-op). Agent-mail and session search exist in core with no UI. Plugin grants/approvals are inert in the renderer. Inbox and most Settings pages are still copy.

No test failures. Do not treat this as a green 0.1.0 ship.

## 2. Commands actually run

From `harbor/` on 2026-09-06. Working directory was the Harbor repo (parent checkout is not a Cargo workspace).

### 2.1 `bash scripts/deny-brand.sh`

```
deny-brand: clean
EXIT:0
```

### 2.2 `cargo test -p harbor-core --locked`

```
running 23 tests
test engines::tests::catalog_is_the_single_table ... ok
test engines::tests::missing_binary_is_cli_missing ... ok
test engines::tests::installed_acp_engine_is_available_for_chat ... ok
test engines::tests::path_search_skips_relative_and_cwd ... ok
test agents::tests::create_maps_unique_name_and_draft_is_local ... ok
test agents::tests::face_preview_is_a_stable_svg_data_url ... ok
test acp::tests::resolve_selects_or_cancels ... ok
test files::tests::read_write_list_stay_inside_workspace ... ok
test restore::tests::cold_start_does_not_spawn_or_resume_processes ... ok
test agents::tests::update_applies_all_fields_and_rejects_bad_names ... ok
test layout::tests::save_tidy_panes_and_restore ... ok
test mail::tests::send_writes_mail_chat_and_notification ... ok
test chats::tests::create_send_history_config_and_cascade ... ok
test memory::tests::upsert_list_delete_and_reject_empty ... ok
test places::tests::grant_canonicalizes_and_skips_dupes ... ok
test tests::host_commands_do_not_require_auth ... ok
test search::tests::searches_agent_prose_not_tool_output ... ok
test tests::settings_roundtrip_and_schema ... ok
test threads::tests::send_persists_user_and_context ... ok
test plugins::tests::connect_has_no_token_and_grants_roundtrip ... ok
test workspaces::tests::reopening_folder_preserves_id_and_pin_and_rejects_invalid_paths ... ok
test threads::tests::set_config_merges_and_attach_files_persist ... ok
test threads::tests::history_is_ordered_and_scoped_to_one_thread_kind ... ok

test result: ok. 23 passed; 0 failed; 0 ignored
Doc-tests harbor_core: ok. 0 passed
EXIT:0
```

### 2.3 `cargo test -p harbor-desktop --locked`

```
running 16 tests
test acp_host::tests::agent_cwd_prefers_home_then_place_then_temp ... ok
test acp_host::tests::extracts_only_visible_assistant_chunks_from_v1_updates ... ok
test crash::tests::panic_child ... ok
test host_config::main_and_overlay_match_pr03_geometry ... ok
test host_config::capabilities_are_least_privilege ... ok
test host_config::minisign_placeholder_cannot_verify_updates ... ok
test host_config::window_icon_is_rgba_png ... ok
test host_config::release_panics_abort_and_no_renderer_grant_commands ... ok
test security::tests::bootstrap_grants_running_binary_as_harbor_not_engine ... ok
test security::tests::navigation_stays_local_and_dev_origin_is_not_in_release ... ok
test security::tests::grants_are_absolute_and_role_specific ... ok
test security::tests::retargeted_symlink_does_not_inherit_grant ... ok
test security::tests::path_search_excludes_cwd_workspace_and_relative_entries ... ok
test crash::tests::installed_hook_records_crash_without_secret_payload ... ok
test acp_host::tests::permission_resolve_updates_row_and_channel ... ok
test acp_host::tests::agent_chat_context_uses_home_and_places ... ok

test result: ok. 16 passed; 0 failed; 0 ignored
EXIT:0
```

### 2.4 `cargo test -p harbor-acp --locked`

```
lib unit: 2 passed (newline-delimited transport)
tests/fake_agent.rs: 7 passed
  load_only_drops_replay_updates
  never_maps_load_session_to_resume
  resume_only_calls_session_resume_never_load
  mcp_env_is_name_value_array_not_object
  neither_opens_new_with_banner
  permissions_echo_option_id_and_cancel
  config_options_and_additional_directories_are_opt_in
tests/stdio_session.rs: 5 passed
  stdio_load_only_drops_replay_and_prompts
  stdio_resume_only_opens_session_resume
  stdio_permissions_default_cancel_completes_prompt
  stdio_neither_uses_session_new
  stdio_permissions_hook_echoes_option_id
EXIT:0
```

### 2.5 `cargo clippy -p harbor-core -p harbor-desktop --all-targets --locked -- -D warnings`

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
EXIT:0
```

Cached clippy profile; no diagnostics. Re-ran after tests; still 0.

### 2.6 `pnpm --filter @harbor/desktop test`

```
✓ src/ui-structure.test.tsx (2)
✓ src/modes/agent/AgentPage.test.tsx (6)
✓ src/modes/chat/useAcpThread.test.tsx (5)
✓ src/modes/chat/ChatMode.test.tsx (5)
Test Files  4 passed (4)
Tests  18 passed (18)
EXIT:0
```

### 2.7 `pnpm --filter @harbor/ui check`

Script exists (`tsc --noEmit && vite build ./preview`).

```
tsc --noEmit && vite build ./preview --outDir ../dist/preview --emptyOutDir
✓ 39 modules transformed
✓ built in 590ms
EXIT:0
```

## 3. Pass-1 fakes — now real

| Check | Result |
| --- | --- |
| AgentPage uses `agent_chat_list/create/send/history/cancel` (not local `chat-1`) | **Real.** `useAgentChat` invokes those commands. Vitest asserts no `chat-1` tab. |
| GearPanel memory + `places_list` | **Real.** `memory_list/upsert/delete` and `places_list/grant/revoke`. Tests cover list/upsert/revoke. |
| PermissionCard mounted; `acp_permission_resolve` | **Real.** Mounted in Agent and Chat. Host sets a permission hook, emits `acp_permission`, resolve writes the row and the pending channel. |
| `thread_send` persists + ACP | **Real.** Core inserts the user line; host then `acp_host::prompt`. Assistant prose persisted; `acp_update` emitted. |
| `agent_chat_send` persists + ACP | **Real.** Core `chats::send` then host `prompt_agent`. |
| Appearance loaded on boot and footer persists | **Real.** `App` reads `settings.appearance`. Footer `Segmented` writes `appearance`. Settings General does too. |
| deny-brand clean | **Real.** See §2.1. |
| `face_preview` / `agent_draft_with_ai` reachable from UI | **Real.** Gear face picker invokes `face_preview`. New Agent “Create with AI” invokes `agent_draft_with_ai`. |
| `workspace_save_layout` / `workspace_tidy` / `layout_restore` exist in core | **Real as commands.** Implemented in `layout.rs`, IPC registered, Code mode invokes them. **Not** an end-to-end restore (see §6). |

### 3.1 Agent chats

`AgentPage` no longer seeds `{ id: "chat-1" }`. It uses `useAgentChat(agent.id)`:

```40:41:apps/desktop/src/modes/agent/useAgentChat.ts
      const listed = await invoke<AgentChat[]>("agent_chat_list", { agentId });
      setChats(listed);
```

Send / create / history / cancel:

```161:161:apps/desktop/src/modes/agent/useAgentChat.ts
      const created = await invoke<AgentChat>("agent_chat_create", { agentId });
```

```62:62:apps/desktop/src/modes/agent/useAgentChat.ts
      const lines = await invoke<ChatMessage[]>("agent_chat_history", { chatId: id });
```

```202:202:apps/desktop/src/modes/agent/useAgentChat.ts
      await invoke("agent_chat_send", { chatId, parts });
```

```222:222:apps/desktop/src/modes/agent/useAgentChat.ts
      await invoke("agent_chat_cancel", { chatId });
```

Host send persists then prompts ACP:

```406:417:apps/desktop/src-tauri/src/ipc.rs
pub async fn agent_chat_send(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    chat_id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::agent_chat_send(&pool, chat_id.clone(), parts.clone())
        .await
        .map_err(map_err)?;
    crate::acp_host::prompt_agent(&app, &pool, &allow, &registry, &chat_id, &parts).await
}
```

### 3.2 Gear memory / places

```24:29:apps/desktop/src/modes/agent/GearPanel.tsx
    void invoke<Memory[]>("memory_list", { agentId: agent.id })
      .then(setMemories)
      .catch(() => setMemories([]));
    void invoke<Place[]>("places_list", { agentId: agent.id })
      .then(setPlaces)
      .catch(() => setPlaces([]));
```

### 3.3 Permission cards

Mounted on both modes:

```87:92:apps/desktop/src/modes/agent/AgentPage.tsx
              {chat.permissions.map((request) => (
                <PermissionCard
                  key={request.id}
                  request={request}
                  onResolve={(optionId, cancelled) => void chat.resolvePermission(request.id, optionId, cancelled)}
                />
```

```164:169:apps/desktop/src/modes/chat/ChatMode.tsx
        {acp.permissions.map((request) => (
          <PermissionCard
            key={request.id}
            request={request}
            onResolve={(optionId, cancelled) => void acp.resolvePermission(request.id, optionId, cancelled)}
          />
```

Host hook is set (not auto-cancel-only):

```351:352:apps/desktop/src-tauri/src/acp_host.rs
    let mut session = AcpHostSession::connect(spec.clone()).map_err(|error| error.to_string())?;
    session.set_permission_hook(hook);
```

Timeout still cancels after 300s if the builder never answers. That is a fallback, not the only path.

### 3.4 Chat `thread_send`

```274:286:apps/desktop/src-tauri/src/ipc.rs
pub async fn thread_send(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    allow: State<'_, ExecutableAllowlist>,
    registry: State<'_, AcpRegistry>,
    id: String,
    parts: Vec<ContentPart>,
) -> Result<(), String> {
    harbor_core::commands::thread_send(&pool, id.clone(), parts.clone())
        .await
        .map_err(map_err)?;
    crate::acp_host::prompt(&app, &pool, &allow, &registry, &id, &parts).await
}
```

`threads::send` inserts the user message and updates the title. After the ACP turn, `run_turn` appends assistant prose and emits `acp_update`. Resume banner on `FreshWithBanner`. `mcp_servers: vec![]` always (allowed).

### 3.5 Appearance

Boot:

```14:24:apps/desktop/src/App.tsx
  useEffect(() => {
    void Promise.all([
      settingsGet("onboarded_local"),
      settingsGet("local_profile_name"),
      settingsGet("appearance"),
    ]).then(([onboardedValue, name, appearance]) => {
      setOnboarded(onboardedValue === true);
      if (typeof name === "string" && name.trim()) setProfileName(name);
      if (appearance === "black" || appearance === "light") setTheme(appearance);
      setReady(true);
    });
  }, []);
```

Footer (mounted in `AppRail`):

```17:27:apps/desktop/src/chrome/Footer.tsx
      <Segmented
        label="Appearance"
        value={theme}
        options={[
          { value: "black", label: "Black" },
          { value: "light", label: "Light" },
        ]}
        onValueChange={(value) => {
          onThemeChange(value);
          void settingsSet("appearance", value);
        }}
      />
```

Dark-before-paint still holds: `index.html` `data-theme="black"` + `#0B0B0C`; `main.tsx` `forceDark()`; window `backgroundColor` `#0B0B0C`.

### 3.6 Face preview / draft with AI

Gear picker:

```84:91:apps/desktop/src/modes/agent/GearPanel.tsx
  async function chooseFace(index: number) {
    const previous = faceIndex;
    setFaceIndex(index);
    setError(null);
    try {
      await invoke("agent_update", { input: { id: agent.id, faceIndex: index } });
      await invoke<string>("face_preview", { agentId: agent.id, faceIndex: index });
```

New Agent:

```50:52:apps/desktop/src/modes/agent/NewAgent.tsx
      const drafted = await invoke<{ name: string; brief: string; engineId?: string }>("agent_draft_with_ai", {
        hint: brief.trim() || name.trim() || "coding teammate",
      });
```

`draft_with_ai` is a local unique-name helper (default engine `opencode`), not a networked model. Honest and reachable.

Schema still types `face_preview` as `{ pngB64 }`. Core returns an SVG data URL string. The UI treats it as `string` and discards the value after invoke. Reachable, contract mismatch remains.

### 3.7 Layout commands in core

```49:63:crates/harbor-core/src/commands.rs
pub async fn workspace_save_layout(
    pool: &SqlitePool,
    tab_id: String,
    layout: PaneLayout,
) -> Result<(), Error> {
    crate::layout::save(pool, &tab_id, &layout).await
}

pub async fn workspace_tidy(pool: &SqlitePool, tab_id: String) -> Result<PaneLayout, Error> {
    crate::layout::tidy(pool, &tab_id).await
}

pub async fn layout_restore(pool: &SqlitePool) -> Result<(), Error> {
    crate::layout::restore(pool).await
}
```

`layout::save` UPDATEs `workspace_tabs.layout_json`. `tidy` renormalizes split ratios by leaf count and writes back. `restore` SELECTs tab ids and `UPDATE panes SET paused = 1 WHERE kind = 'terminal'`. Unit test `save_tidy_panes_and_restore` passes.

## 4. Design vs DESIGN.md §5–6

| Check | Result |
| --- | --- |
| Forced dark before first paint | **Pass.** |
| `--harbor-*` tokens | **Pass.** bg/surface/raised/border/text/muted, speak/think/error, live, needs-you, rail 280, titlebar 44, orb 28, composer min 72. Light block present. |
| Title-bar order (K30) | **Mostly pass vs pass 1.** Shipped: wordmark + mode, flex, **Workspace ▾** (`WorkspaceMenu` mounted), Orb, Bell, sidebar, Windows controls. Lights are native Overlay traffic lights. Spec “logo directly behind window buttons” is Overlay geometry, not a second in-content lights cluster. |
| Inactive mode = visibility / content-visibility | **Pass.** `.harbor-mode` uses `visibility` + `content-visibility` + `pointer-events`. Destinations hide `.harbor-stage-panel` the same way. |
| Copy | **Pass (case).** Welcome CTA `Start local`, secondary `Local profile`, composer `Ask anything...`. Tagline: `An open desktop host for coding agents` (spec lowercase *an*). Orb: `Voice is off until a later release`. Footer `Free · local`. No Credits. |
| Light theme persist | **Pass.** Boot read + footer/settings write. |
| Linux native decorations + in-content toolbar | **Fail.** `titleBarStyle: Overlay` for all platforms. Host test asserts Overlay. `decorations: true`. |
| Rail destinations | **Fail vs changelog/K.** `Destinations.tsx` still puts Dashboard/Plugins **rows at the top of the rail**. Spec: rail = mode list; destinations in footer/command. |
| Faces | Atlas is used. Slot = `faceIndex % 64`, not `blake3(agent_uuid) % 64`. Picker is 12 atlas cells. Core preview is 12 SVG palettes, unused by the atlas Face. |
| Orb | Mute 28px CSS disc. Allowed mute-chrome for 0.1.0. Not plasma/orbit layers. |
| Inter | `@font-face { src: local("Inter…") }` only. No bundled webfont. |
| Settings | Five pages exist. General appearance is live. Notifications/Voice/Agents/Account are mostly explanatory copy. Voice buttons call dictation commands (begin/end unimplemented). |
| Inbox | Static `{agent} is waiting on a prompt` templates. Not live events. |
| Composer High chip | Agent composer uses ACP `configOptions` via `ModelMenu` when present. No hardcoded High chip. |
| Paywall | **Absent.** Welcome `Start local`. Account `Free · local`. UI test asserts no `Credits`. |

## 5. Remaining `unimplemented`

Grep of `crates/` and `apps/desktop/src-tauri` (`*.rs`):

**`harbor-core` `Error::unimplemented` (intentional host-owned or later):**

| Command | Notes |
| --- | --- |
| `pty_spawn/write/resize/pause/resume/kill` | Core stubs. Host implements spawn/write/resize/kill. |
| `dictation_begin/end/devices/prepare_model` | Core stubs. Host: begin/end still unimplemented; devices empty-success via speech crate; prepare_model errors “Whisper model is not bundled”. |
| `updater_check/install` | Core stubs. Host `updater_check` uses GitHub Releases fetch + placeholder minisign key → never `available: true`. `updater_install` refuses. |

**Host still unimplemented:**

```599:606:apps/desktop/src-tauri/src/ipc.rs
pub async fn dictation_begin() -> Result<(), String> {
    Err("unimplemented: dictation_begin".into())
}

pub async fn dictation_end() -> Result<(), String> {
    Err("unimplemented: dictation_end".into())
}
```

**Host no-ops (not unimplemented, but not pause):**

```168:176:apps/desktop/src-tauri/src/pty_host.rs
pub fn pty_pause(_pane_id: String) -> Result<(), String> {
    Ok(())
}

pub fn pty_resume(_pane_id: String) -> Result<(), String> {
    Ok(())
}
```

No remaining `unimplemented` on `agent_chat_*`, `memory_*`, `places_*`, `layout_*`, `workspace_save_layout`, `workspace_tidy`, `acp_permission_resolve`, `face_preview`, `agent_draft_with_ai`, `thread_send`, `thread_set_config`, `thread_attach_files`, `mail_send`, `session_search`, `pane_create/close`.

## 6. Host / renderer wiring that is still incomplete

### 6.1 Restore is not end-to-end (0.1.0 DoD item 5)

- `workspace_add` does **not** insert `workspace_tabs`. The only INSERT in product code is the layout unit test.
- `layout::save` is `UPDATE workspace_tabs … WHERE id = ?`. Missing tab → `"tab not found"`.
- Code mode uses `workspace.id` as `tabId` (or a setting `code_tab:${workspace.id}`) and `catch`es persist failures. On a fresh DB, save/tidy no-op.
- `layout_restore` only marks existing `panes` paused. Code mode never calls `pane_create`; leaves are hardcoded `"term"` / `"files"`.
- Renderer `layout/restore.ts` is identity (`restoredLayoutPaused` returns the input). Nothing loads saved `layout_json` into Code mode state.
- Code mode `paused` starts `false`. Restore never sets it. Resume chrome exists but is unused.

So the commands exist; a cold start does not restore layout, tabs, or paused terminals.

### 6.2 Agent-mail and session search (0.1.0 DoD item 4)

Core + tests exist (`mail::send` writes a Mail chat + notification; FTS5 search skips tool output). **No renderer `invoke("session_search")` or `invoke("mail_send")`.** Gear has memory/places only.

### 6.3 Plugin grants / approvals

`plugin_connect` host: GitHub Device Flow, empty baked `CLIENT_ID`, requires `HARBOR_GITHUB_CLIENT_ID` or settings. No `client_secret`. Token to keyring with 0600 file fallback. Emits `plugin_device`. Honest (no fake tokens).

`ApprovalCard` only renders Allow/Deny when `id` is passed. `Plugins.tsx` mounts `<ApprovalCard />` with no id → inert copy. No UI calls `plugin_set_agent_grant`. Core grant/approval APIs exist.

### 6.4 Code panes

Solo + Terminal + Files is the 0.1.0 pane set. Command bar sends to focused PTY (`pty_write_b64` pane `"term"`). Split/close in the header only toggles local `closed` / focus; they do not call `pane_create` / `pane_close`. Tidy: local `ratio = 0.5` immediately, then `workspace_tidy` if a tab id exists (usually it does not).

PTY spawn/write/resize/kill is live (`pty-data` b64). Default `paneId = "term"`. cwd = first workspace folder or `"."`.

### 6.5 Dictation / updater (allowed)

- `harbor_speech::engine_available() -> false`. begin/end unimplemented in host. Overlay window exists in `tauri.conf.json` (`visible: false`).
- Updater: placeholder key file says it must never authorize an update. `verify_release` returns PlaceholderKey/Unsigned. `updater_check` therefore reports `available: false`. `updater_install` refuses. No Settings Update row.

### 6.6 MCP inject

`SpawnSpec { mcp_servers: vec![], … }` in `acp_host`. Allowed empty. Plugin tokens are not injected into engines.

## 7. Blockers vs remaining debt

### Blockers for calling Harbor 0.1.0 DoD done

These are still product gaps against DESIGN 0.1.0, **not** in the allowed post-0.1.0 / host-limited list:

1. **Session restore** — commands exist; UI/DB never create tabs; layout JSON is not loaded; restored terminals are not paused.
2. **Agent-mail UI** — core only.
3. **`session_search` UI** — core only.
4. **Plugin approval/grant UI** — Allow/Deny never shown; no per-agent grant toggle.
5. **Linux title bar** — Overlay, not native + in-content toolbar.

No failing tests. No remaining pass-1 fake commands on the assigned list.

### Remaining debt (not 0.1.0 blockers, or allowed)

- `pty_*` unimplemented in core (host owns PTY) — **allowed**. Host pause/resume still no-ops (see blocker 1).
- Dictation begin/end unimplemented; no speech engine — **allowed**.
- Updater cannot install without a real pubkey — **allowed**. Host check is honest refuse.
- `mcpServers` empty — **allowed**.
- Routines / Skills pages say after 0.1.0 — **allowed**.
- No paywall — **held**.
- Inbox static templates.
- Settings pages 2–5 mostly copy (zoom, startup mode, shell, Recheck, Reduce Motion, logs, notification filters not wired).
- Face slot is `index % 64`; schema `{ pngB64 }` vs SVG string.
- Inter not bundled.
- Mute orb is a CSS disc (allowed mute-chrome).
- Dashboard/Plugins still extra rail rows.
- Signed/notarized macOS universal build and Windows PTY CI artifact were **not** re-verified this pass (debug `Harbor.app` exists under `target/`; not an acceptance run).
- `home_path` is written as `""` on create; ACP agent cwd then falls back to a granted place or temp.

## 8. Allowed exceptions (confirmed)

| Item | Evidence |
| --- | --- |
| `pty_*` unimplemented in core | `commands.rs` 78–104. Host spawn/write/resize/kill live. |
| Dictation begin/end unimplemented | Host `ipc.rs` 599–606; `engine_available() == false`. |
| Updater cannot install without real pubkey | Placeholder minisign file; `refuse_install`; check never available. |
| `mcpServers` empty | `acp_host.rs` `mcp_servers: vec![]`. |
| Routines/Skills after 0.1.0 | Destination empty states + Agent Skills tab copy. |
| No paywall | Welcome/Account/footer; no Credits. |
