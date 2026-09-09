/**
 * IPC contract for Harbor 0.1.0. The payload shapes mirror the types in
 * `crates/harbor-core/src/types.rs`; the command list at the bottom mirrors
 * `generate_handler!` in `apps/desktop/src-tauri/src/ipc.rs`. A Rust test
 * there fails when the two drift, so edit both sides together.
 */

export interface Workspace {
  id: string;
  folder: string;
  title: string | null;
  pinned: boolean;
}

export interface WorkspaceSetup {
  additionalTerminals: number;
  browserPreview: boolean;
  threadPane: boolean;
  terminalEngineIds: string[];
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "mail";
  text: string;
}

export interface DetectedEngine {
  id: string;
  displayName: string;
  path: string;
  status: string;
  supportsChat: boolean;
  supportsTerminal: boolean;
  /** Set when chat needs an adapter this machine does not have yet. */
  adapterPackage?: string | null;
}

/** The pane kinds Code mode can create. */
export type PaneKind = "terminal" | "files" | "browser" | "thread";

export type PaneLayout =
  | { type: "leaf"; paneId: string }
  | { type: "split"; dir: string; ratio: number; a: PaneLayout; b: PaneLayout }
  | { type: "tabs"; active: number; kids: string[] };

export interface PaneState {
  kind: string;
  cwd?: string | null;
  paused?: boolean | null;
  engineId?: string | null;
}

export interface ThreadRecord {
  id: string;
  workspaceId: string | null;
  title: string;
  engineId: string;
  pinned: boolean;
  unread: boolean;
}

export type AgentTrailing =
  | { kind: "running"; n: number }
  | { kind: "needs_you" }
  | { kind: "last_spoke"; at: number }
  | { kind: "idle" };

export interface AgentRecord {
  id: string;
  name: string;
  brief: string;
  engineId: string;
  faceIndex: number;
  pinned: boolean;
  homePath?: string;
  messaging?: boolean;
  lastLine?: string | null;
  trailing?: AgentTrailing;
}

export interface CreateAgent {
  name: string;
  brief: string;
  engineId: string;
  faceIndex?: number;
}

export interface UpdateAgent {
  id: string;
  name?: string | null;
  brief?: string | null;
  engineId?: string | null;
  faceIndex?: number | null;
  pinned?: boolean | null;
  homePath?: string | null;
  messaging?: boolean | null;
}

export interface AgentChat {
  id: string;
  agentId: string;
  title: string;
  status: string;
}

export interface Memory {
  id: string;
  body: string;
  kind: string;
}

export interface Place {
  id: string;
  path: string;
}

export interface SearchHit {
  chat_id: string;
  prose: string;
  created_at: number;
}

export interface PluginRow {
  id: string;
  displayName: string;
  status: string;
  accountLabel?: string | null;
  description: string;
  category: string;
  authKind: "device" | "token" | string;
}

export interface PluginGrant {
  pluginId: string;
  enabled: boolean;
}

export interface PluginApproval {
  id: string;
  pluginId: string;
  agentId: string | null;
  action: string;
  status: string;
}

export interface RestoredPane {
  id: string;
  kind: string;
  paused: boolean;
  engineId?: string | null;
}

export interface WorkspaceTab {
  id: string;
  workspaceId: string;
  layout: PaneLayout;
  panes: RestoredPane[];
}

export interface FsEntry {
  name: string;
  path: string;
  directory: boolean;
}

export interface FileDiff {
  path: string;
  patch: string;
}

export interface UpdateStatus {
  available: boolean;
  version: string | null;
}

export interface AudioDevice {
  id: string;
  label: string;
  selected: boolean;
}

export interface ContentPart {
  type: string;
  text?: string | null;
  path?: string | null;
}

export interface Notification {
  id: string;
  kind: string;
  title: string;
  body: string;
  read: boolean;
  createdAt: number;
  mode: string | null;
  workspaceId: string | null;
  paneId: string | null;
  sessionRef: string | null;
}

export interface EngineIcon {
  engineId: string;
  dataUrl: string;
}

/**
 * Every command the host registers, with the argument object the webview sends
 * and what the host answers. `args: undefined` marks a command that takes none.
 *
 * `call()` in the desktop app is the only way this list is used, so a typo
 * cannot reach the host, and `contract_matches_the_registered_commands` in
 * `ipc.rs` fails the build when Rust and this file disagree on the set of
 * commands or on an argument name.
 */
export interface HarborCommands {
  settings_get: { args: { key: string }; returns: unknown };
  settings_set: { args: { key: string; value: unknown }; returns: void };
  default_profile_name: { args: undefined; returns: string };

  engines_detect: { args: undefined; returns: DetectedEngine[] };
  engines_recheck: { args: undefined; returns: DetectedEngine[] };
  engine_icons: { args: undefined; returns: EngineIcon[] };
  engine_install_adapter: { args: { engineId: string }; returns: DetectedEngine[] };

  workspace_list: { args: undefined; returns: Workspace[] };
  workspace_pick_folder: { args: undefined; returns: string | null };
  workspace_add: { args: { folder: string }; returns: Workspace };
  workspace_remove: { args: { id: string }; returns: void };
  workspace_pin: { args: { id: string; pinned: boolean }; returns: void };
  workspace_rename: { args: { id: string; title: string }; returns: Workspace };
  workspace_save_layout: { args: { tabId: string; layout: PaneLayout }; returns: void };
  workspace_tidy: { args: { tabId: string }; returns: PaneLayout };
  workspace_ensure_tab: { args: { workspaceId: string }; returns: WorkspaceTab };
  workspace_configure_tab: { args: { workspaceId: string; setup: WorkspaceSetup }; returns: WorkspaceTab };
  layout_restore: { args: undefined; returns: WorkspaceTab[] };

  pane_create: { args: { tabId: string; kind: PaneKind; state: PaneState }; returns: string };
  pane_close: { args: { id: string }; returns: void };
  pane_set_engine: { args: { id: string; engineId: string }; returns: void };

  pty_spawn: {
    args: { paneId: string; workspaceId: string; cols: number; rows: number; shell?: string | null; engineId?: string | null };
    returns: void;
  };
  pty_write_b64: { args: { paneId: string; b64: string }; returns: void };
  pty_resize: { args: { paneId: string; cols: number; rows: number }; returns: void };
  pty_pause: { args: { paneId: string }; returns: void };
  pty_resume: { args: { paneId: string }; returns: void };
  pty_kill: { args: { paneId: string }; returns: void };

  fs_read: { args: { workspaceId: string; path: string }; returns: string };
  fs_write: { args: { workspaceId: string; path: string; contents: string }; returns: void };
  fs_list: { args: { workspaceId: string; path: string }; returns: FsEntry[] };
  git_diff: { args: { workspaceId: string }; returns: FileDiff[] };

  thread_list: { args: { workspaceId: string | null }; returns: ThreadRecord[] };
  thread_history: { args: { id: string }; returns: ChatMessage[] };
  thread_create: { args: { workspaceId: string | null; engineId: string }; returns: ThreadRecord };
  thread_rename: { args: { id: string; title: string }; returns: void };
  thread_delete: { args: { id: string }; returns: void };
  thread_pin: { args: { id: string; pinned: boolean }; returns: void };
  thread_send: { args: { id: string; parts: ContentPart[] }; returns: void };
  thread_cancel: { args: { id: string }; returns: void };
  thread_set_engine: { args: { id: string; engineId: string }; returns: void };
  /** Shaped by the engine, so the caller parses it rather than trusting a type. */
  thread_config_options: { args: { id: string }; returns: unknown };
  thread_set_config: { args: { id: string; optionId: string; value: unknown }; returns: void };
  thread_grant_root: { args: { id: string; path: string }; returns: void };
  thread_attach_files: { args: { id: string; paths: string[] }; returns: void };

  agent_list: { args: undefined; returns: AgentRecord[] };
  agent_create: { args: { input: CreateAgent }; returns: AgentRecord };
  agent_update: { args: { input: UpdateAgent }; returns: void };
  agent_delete: { args: { id: string }; returns: void };
  agent_draft_with_ai: { args: { hint: string }; returns: CreateAgent };
  agent_chat_list: { args: { agentId: string }; returns: AgentChat[] };
  agent_chat_create: { args: { agentId: string }; returns: AgentChat };
  agent_chat_rename: { args: { chatId: string; title: string }; returns: void };
  agent_chat_delete: { args: { chatId: string }; returns: void };
  agent_chat_history: { args: { chatId: string }; returns: ChatMessage[] };
  agent_chat_send: { args: { chatId: string; parts: ContentPart[] }; returns: void };
  agent_chat_cancel: { args: { chatId: string }; returns: void };
  agent_chat_config_options: { args: { chatId: string }; returns: unknown };
  agent_chat_set_config: { args: { chatId: string; optionId: string; value: unknown }; returns: void };

  memory_list: { args: { agentId: string }; returns: Memory[] };
  memory_upsert: { args: { agentId: string; body: string }; returns: Memory };
  memory_delete: { args: { id: string }; returns: void };
  places_list: { args: { agentId: string }; returns: Place[] };
  places_grant: { args: { agentId: string; path: string }; returns: void };
  places_revoke: { args: { id: string }; returns: void };
  session_search: { args: { agentId: string; query: string }; returns: SearchHit[] };
  /** Tells the host which conversation is on screen, so its replies skip the inbox. */
  session_watch: { args: { sessionRef: string | null }; returns: void };
  mail_send: { args: { fromAgentId: string; toAgentId: string; body: string }; returns: void };
  notifications_list: { args: undefined; returns: Notification[] };
  notifications_mark_read: { args: undefined; returns: void };
  notifications_unread_count: { args: undefined; returns: number };
  /** A data URL for the generated face, not the raw PNG bytes. */
  face_preview: { args: { agentId: string; faceIndex: number }; returns: string };

  acp_permission_resolve: { args: { id: string; optionId: string | null; cancelled: boolean }; returns: void };

  plugin_list: { args: undefined; returns: PluginRow[] };
  plugin_connect: { args: { id: string }; returns: void };
  plugin_configure: { args: { id: string; credential: string; accountLabel?: string | null }; returns: void };
  plugin_disconnect: { args: { id: string }; returns: void };
  plugin_set_agent_grant: { args: { agentId: string; pluginId: string; enabled: boolean }; returns: void };
  plugin_resolve_approval: { args: { id: string; allow: boolean }; returns: void };
  plugin_approvals_list: { args: undefined; returns: PluginApproval[] };
  plugin_grants_list: { args: { agentId: string }; returns: PluginGrant[] };

  dictation_begin: { args: undefined; returns: void };
  dictation_end: { args: undefined; returns: void };
  dictation_devices: { args: undefined; returns: AudioDevice[] };
  dictation_prepare_model: { args: undefined; returns: void };

  updater_check: { args: undefined; returns: UpdateStatus };
  updater_install: { args: undefined; returns: void };
}

export type HarborCommand = keyof HarborCommands;
export type CommandArgs<K extends HarborCommand> = HarborCommands[K]["args"];
export type CommandResult<K extends HarborCommand> = HarborCommands[K]["returns"];
