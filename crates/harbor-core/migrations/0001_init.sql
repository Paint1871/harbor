CREATE TABLE agents (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL UNIQUE,
  brief         TEXT NOT NULL DEFAULT '',
  engine_id     TEXT NOT NULL,
  face_index    INTEGER NOT NULL DEFAULT 0,
  home_path     TEXT NOT NULL,
  memory_budget INTEGER NOT NULL DEFAULT 64,
  reflection    TEXT NOT NULL DEFAULT 'off',
  messaging     INTEGER NOT NULL DEFAULT 0,
  routines_ok   INTEGER NOT NULL DEFAULT 0,
  pinned        INTEGER NOT NULL DEFAULT 0,
  pin_order     INTEGER,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);

CREATE TABLE agent_chats (
  id            TEXT PRIMARY KEY,
  agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  title         TEXT NOT NULL DEFAULT 'New chat',
  status        TEXT NOT NULL DEFAULT 'idle',
  tab_order     INTEGER NOT NULL,
  acp_session   TEXT,
  config_json   TEXT NOT NULL DEFAULT '{}',
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);
CREATE INDEX idx_agent_chats_agent ON agent_chats(agent_id, tab_order);

CREATE TABLE messages (
  id            TEXT PRIMARY KEY,
  chat_id       TEXT NOT NULL,
  chat_kind     TEXT NOT NULL,
  role          TEXT NOT NULL,
  prose         TEXT NOT NULL DEFAULT '',
  payload_json  TEXT NOT NULL DEFAULT '{}',
  created_at    INTEGER NOT NULL
);
CREATE INDEX idx_messages_chat ON messages(chat_kind, chat_id, created_at);

CREATE VIRTUAL TABLE messages_fts USING fts5(
  prose,
  content='messages',
  content_rowid='rowid'
);

CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, prose) VALUES (new.rowid, new.prose);
END;
CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, prose) VALUES('delete', old.rowid, old.prose);
END;
CREATE TRIGGER messages_au AFTER UPDATE OF prose ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, prose) VALUES('delete', old.rowid, old.prose);
  INSERT INTO messages_fts(rowid, prose) VALUES (new.rowid, new.prose);
END;

CREATE TABLE acp_permissions (
  id            TEXT PRIMARY KEY,
  session_ref   TEXT NOT NULL,
  session_kind  TEXT NOT NULL,
  tool_title    TEXT,
  path          TEXT,
  command       TEXT,
  options_json  TEXT NOT NULL,
  status        TEXT NOT NULL,
  selected_option_id TEXT,
  created_at    INTEGER NOT NULL
);

CREATE TABLE memories (
  id            TEXT PRIMARY KEY,
  agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  body          TEXT NOT NULL,
  kind          TEXT NOT NULL DEFAULT 'fact',
  created_at    INTEGER NOT NULL
);

CREATE TABLE skills (
  id            TEXT PRIMARY KEY,
  slug          TEXT NOT NULL UNIQUE,
  title         TEXT NOT NULL,
  body_md       TEXT NOT NULL,
  starter       INTEGER NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL
);

CREATE TABLE agent_skills (
  agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  skill_id      TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  PRIMARY KEY (agent_id, skill_id)
);

CREATE TABLE places (
  id            TEXT PRIMARY KEY,
  agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  path          TEXT NOT NULL,
  granted_at    INTEGER NOT NULL
);

CREATE TABLE routines (
  id            TEXT PRIMARY KEY,
  agent_id      TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  name          TEXT NOT NULL,
  instruction   TEXT NOT NULL,
  schedule_json TEXT NOT NULL,
  timezone      TEXT NOT NULL,
  paused        INTEGER NOT NULL DEFAULT 0,
  next_run_at   INTEGER,
  last_chat_id  TEXT,
  last_error    TEXT,
  created_at    INTEGER NOT NULL
);

CREATE TABLE routine_runs (
  id            TEXT PRIMARY KEY,
  routine_id    TEXT NOT NULL REFERENCES routines(id) ON DELETE CASCADE,
  chat_id       TEXT,
  started_at    INTEGER NOT NULL,
  finished_at   INTEGER,
  status        TEXT NOT NULL,
  error         TEXT
);

CREATE TABLE workspaces (
  id            TEXT PRIMARY KEY,
  folder        TEXT NOT NULL UNIQUE,
  title         TEXT,
  pinned        INTEGER NOT NULL DEFAULT 0,
  pin_order     INTEGER,
  last_opened   INTEGER
);

CREATE TABLE workspace_tabs (
  id            TEXT PRIMARY KEY,
  workspace_id  TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  title         TEXT NOT NULL,
  tab_order     INTEGER NOT NULL,
  flipped       INTEGER NOT NULL DEFAULT 0,
  layout_json   TEXT NOT NULL,
  rail_width    INTEGER NOT NULL DEFAULT 280,
  tree_width    INTEGER,
  created_at    INTEGER NOT NULL
);

CREATE TABLE panes (
  id            TEXT PRIMARY KEY,
  tab_id        TEXT NOT NULL REFERENCES workspace_tabs(id) ON DELETE CASCADE,
  kind          TEXT NOT NULL,
  state_json    TEXT NOT NULL DEFAULT '{}',
  paused        INTEGER NOT NULL DEFAULT 1,
  created_at    INTEGER NOT NULL
);

CREATE TABLE worktree_lanes (
  id            TEXT PRIMARY KEY,
  workspace_id  TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  slug          TEXT NOT NULL,
  branch        TEXT NOT NULL,
  path          TEXT NOT NULL,
  seat_id       TEXT,
  status        TEXT NOT NULL,
  created_at    INTEGER NOT NULL
);

CREATE TABLE threads (
  id            TEXT PRIMARY KEY,
  workspace_id  TEXT,
  title         TEXT NOT NULL DEFAULT 'New thread',
  engine_id     TEXT NOT NULL,
  git_branch    TEXT,
  pinned        INTEGER NOT NULL DEFAULT 0,
  unread        INTEGER NOT NULL DEFAULT 0,
  acp_session   TEXT,
  config_json   TEXT NOT NULL DEFAULT '{}',
  extra_roots_json TEXT NOT NULL DEFAULT '[]',
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);
CREATE INDEX idx_threads_ws ON threads(workspace_id, updated_at DESC);

CREATE TABLE plugins (
  id            TEXT PRIMARY KEY,
  display_name  TEXT NOT NULL,
  status        TEXT NOT NULL,
  account_label TEXT,
  connected_at  INTEGER
);

CREATE TABLE plugin_grants (
  id            TEXT PRIMARY KEY,
  plugin_id     TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
  agent_id      TEXT REFERENCES agents(id) ON DELETE CASCADE,
  enabled       INTEGER NOT NULL DEFAULT 0,
  scopes_json   TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE plugin_approvals (
  id            TEXT PRIMARY KEY,
  plugin_id     TEXT NOT NULL,
  agent_id      TEXT,
  action        TEXT NOT NULL,
  payload_json  TEXT NOT NULL,
  status        TEXT NOT NULL,
  created_at    INTEGER NOT NULL
);

CREATE TABLE prompts (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  body          TEXT NOT NULL,
  created_at    INTEGER NOT NULL
);

CREATE TABLE launch_presets (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  workspace_id  TEXT,
  seats_json    TEXT NOT NULL,
  isolation     TEXT NOT NULL DEFAULT 'shared',
  created_at    INTEGER NOT NULL
);

CREATE TABLE settings (
  key           TEXT PRIMARY KEY,
  value_json    TEXT NOT NULL
);

CREATE TABLE notifications (
  id            TEXT PRIMARY KEY,
  kind          TEXT NOT NULL,
  title         TEXT NOT NULL,
  body          TEXT NOT NULL,
  target_json   TEXT NOT NULL,
  read_at       INTEGER,
  created_at    INTEGER NOT NULL
);

CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  workspace_id  TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
  title         TEXT NOT NULL,
  status        TEXT NOT NULL DEFAULT 'todo',
  body          TEXT,
  created_at    INTEGER NOT NULL
);
