export type EngineId =
  | "opencode"
  | "claude-code"
  | "codex"
  | "cursor"
  | "grok-build"
  | "gemini"
  | "copilot"
  | "droid"
  | "factory"
  | "kimi-code"
  | "muse-code"
  | "amp"
  | "antigravity"
  | "aider";

export type EngineStatus =
  | "ready"
  | "cli-missing"
  | "adapter-missing"
  | "auth-required"
  | "adapter-protocol-mismatch";

export type ChatMode = "detect" | "adapter" | "none";

export interface EngineSpec {
  id: EngineId;
  displayName: string;
  binaries: string[];
  acpArgs?: string[] | null;
  ptyArgs?: string[];
  supportsTerminal: boolean;
  adapterPackage?: string | null;
  minVersion?: string | null;
  lastHandshake?: string | null;
  authHint: string;
  chatMode: ChatMode;
  workspaceSettingsArg?: string[];
}

export interface DetectedEngine {
  spec: EngineSpec;
  path: string;
  version?: string;
  adapterPath?: string;
  supportsChat: boolean;
  status: EngineStatus;
}
