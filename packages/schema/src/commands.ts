/** IPC contract for Harbor 0.1.0. Generated shape is owned by harbor-core types. */

export interface Workspace {
  id: string;
  folder: string;
  title: string | null;
  pinned: boolean;
}

export interface DetectedEngine {
  id: string;
  displayName: string;
  path: string;
  status: string;
  supportsChat: boolean;
}

export type PaneLayout =
  | { type: "Leaf"; pane_id: string }
  | { type: "Split"; dir: string; ratio: number; a: PaneLayout; b: PaneLayout }
  | { type: "Tabs"; active: number; kids: string[] };

export interface PaneState {
  kind: string;
  cwd?: string | null;
  paused?: boolean | null;
}

export interface ThreadRecord {
  id: string;
  workspaceId: string | null;
  title: string;
  engineId: string;
  pinned: boolean;
  unread: boolean;
}

export interface AgentRecord {
  id: string;
  name: string;
  brief: string;
  engineId: string;
  faceIndex: number;
  pinned: boolean;
}

export interface CreateAgent {
  name: string;
  brief: string;
  engineId: string;
}

export interface UpdateAgent {
  id: string;
  name?: string | null;
  brief?: string | null;
  engineId?: string | null;
  faceIndex?: number | null;
  pinned?: boolean | null;
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

export interface SearchHit {
  chat_id: string;
  prose: string;
  created_at: number;
}

export interface PluginRow {
  id: string;
  displayName: string;
  status: string;
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

export interface ContentPart {
  type: string;
  text?: string | null;
  path?: string | null;
}

export interface HarborCommands {
  settings_get: (key: string) => Promise<unknown>;
  settings_set: (key: string, value: unknown) => Promise<void>;

  engines_detect: () => Promise<DetectedEngine[]>;
  engines_recheck: () => Promise<DetectedEngine[]>;

  workspace_list: () => Promise<Workspace[]>;
  workspace_add: (folder: string) => Promise<Workspace>;
  workspace_remove: (id: string) => Promise<void>;
  workspace_pin: (id: string, pinned: boolean) => Promise<void>;
  workspace_save_layout: (tabId: string, layout: PaneLayout) => Promise<void>;
  workspace_tidy: (tabId: string) => Promise<PaneLayout>;
  layout_restore: () => Promise<void>;

  pane_create: (tabId: string, kind: "terminal" | "files", state: PaneState) => Promise<string>;
  pane_close: (id: string) => Promise<void>;

  pty_spawn: (input: { paneId: string; cwd: string; shell?: string }) => Promise<void>;
  pty_write_b64: (paneId: string, b64: string) => Promise<void>;
  pty_resize: (paneId: string, cols: number, rows: number) => Promise<void>;
  pty_pause: (paneId: string) => Promise<void>;
  pty_resume: (paneId: string) => Promise<void>;
  pty_kill: (paneId: string) => Promise<void>;

  fs_read: (workspaceId: string, path: string) => Promise<string>;
  fs_write: (workspaceId: string, path: string, contents: string) => Promise<void>;
  fs_list: (workspaceId: string, path: string) => Promise<FsEntry[]>;

  thread_list: (workspaceId: string | null) => Promise<ThreadRecord[]>;
  thread_create: (workspaceId: string | null, engineId: string) => Promise<ThreadRecord>;
  thread_rename: (id: string, title: string) => Promise<void>;
  thread_delete: (id: string) => Promise<void>;
  thread_pin: (id: string, pinned: boolean) => Promise<void>;
  thread_send: (id: string, parts: ContentPart[]) => Promise<void>;
  thread_cancel: (id: string) => Promise<void>;
  thread_set_config: (id: string, optionId: string, value: unknown) => Promise<void>;
  thread_grant_root: (id: string, path: string) => Promise<void>;
  thread_attach_files: (id: string, paths: string[]) => Promise<void>;

  agent_list: () => Promise<AgentRecord[]>;
  agent_create: (input: CreateAgent) => Promise<AgentRecord>;
  agent_update: (input: UpdateAgent) => Promise<void>;
  agent_delete: (id: string) => Promise<void>;
  agent_draft_with_ai: (hint: string) => Promise<{ name: string; brief: string }>;
  agent_chat_list: (agentId: string) => Promise<AgentChat[]>;
  agent_chat_create: (agentId: string) => Promise<AgentChat>;
  agent_chat_send: (chatId: string, parts: ContentPart[]) => Promise<void>;
  agent_chat_cancel: (chatId: string) => Promise<void>;
  agent_chat_set_config: (chatId: string, optionId: string, value: unknown) => Promise<void>;
  memory_list: (agentId: string) => Promise<Memory[]>;
  memory_upsert: (agentId: string, body: string) => Promise<Memory>;
  memory_delete: (id: string) => Promise<void>;
  places_grant: (agentId: string, path: string) => Promise<void>;
  places_revoke: (id: string) => Promise<void>;
  session_search: (agentId: string, query: string) => Promise<SearchHit[]>;
  mail_send: (fromAgentId: string, toAgentId: string, body: string) => Promise<void>;
  face_preview: (agentId: string, faceIndex: number) => Promise<{ pngB64: string }>;

  acp_permission_resolve: (id: string, optionId: string | null, cancelled: boolean) => Promise<void>;

  plugin_list: () => Promise<PluginRow[]>;
  plugin_connect: (id: "github") => Promise<void>;
  plugin_disconnect: (id: string) => Promise<void>;
  plugin_set_agent_grant: (agentId: string, pluginId: string, enabled: boolean) => Promise<void>;
  plugin_resolve_approval: (id: string, allow: boolean) => Promise<void>;

  dictation_begin: () => Promise<void>;
  dictation_end: () => Promise<void>;
  dictation_devices: () => Promise<{ id: string; label: string; selected: boolean }[]>;
  dictation_prepare_model: () => Promise<void>;

  updater_check: () => Promise<UpdateStatus>;
  updater_install: () => Promise<void>;

  git_diff: (workspaceId: string) => Promise<FileDiff[]>;
}
