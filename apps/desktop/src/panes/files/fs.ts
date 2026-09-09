import { call } from "../../ipc";
import { readDir, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import type { FsEntry } from "@harbor/schema/commands";

const PRIMARY_TIMEOUT_MS = 2200;
const SCOPED_TIMEOUT_MS = 1800;
const ACCESS_TIMEOUT_MESSAGE = "The folder did not respond. Choose it again from the workspace rail or grant Harbor access in your system privacy settings.";
const RETRY_DELAY_MS = 180;

export function filesystemError(reason: unknown, fallback: string): string {
  const text = String(reason).replace(/^Error:\s*/i, "").trim();
  if (/timed out|permission|denied|not respond/i.test(text)) return ACCESS_TIMEOUT_MESSAGE;
  return text || fallback;
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: number | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = window.setTimeout(() => reject(new Error("filesystem request timed out")), timeoutMs);
  });
  return Promise.race([promise, timeout]).finally(() => {
    if (timer !== undefined) window.clearTimeout(timer);
  });
}

async function retry<T>(operation: () => Promise<T>, attempts = 2): Promise<T> {
  let lastError: unknown;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      return await operation();
    } catch (reason) {
      lastError = reason;
      if (attempt + 1 < attempts) await new Promise((resolve) => window.setTimeout(resolve, RETRY_DELAY_MS));
    }
  }
  throw lastError ?? new Error("filesystem request failed");
}

function isAbsolutePath(value: string): boolean {
  return value.startsWith("/") || /^[A-Za-z]:[\\/]/.test(value) || value.startsWith("\\\\");
}

function lexicalNormalize(value: string): string {
  const slash = value.replaceAll("\\", "/");
  const drive = /^[A-Za-z]:/.test(slash) ? slash.slice(0, 2) : "";
  const absolute = slash.startsWith("/") || Boolean(drive);
  const parts = slash.split("/");
  const output: string[] = [];
  for (const part of parts) {
    if (!part || part === ".") continue;
    if (part === "..") {
      if (output.length && output[output.length - 1] !== "..") output.pop();
      else if (!absolute) output.push(part);
      continue;
    }
    output.push(part);
  }
  const prefix = drive ? `${drive}/` : absolute ? "/" : "";
  return `${prefix}${output.join("/")}`.replace(/\/$/, "") || (drive ? `${drive}/` : "/");
}

function isWithin(root: string, candidate: string): boolean {
  const normalizedRoot = lexicalNormalize(root);
  const normalizedCandidate = lexicalNormalize(candidate);
  const caseInsensitive = /^[A-Za-z]:/.test(normalizedRoot);
  const compareRoot = caseInsensitive ? normalizedRoot.toLowerCase() : normalizedRoot;
  const compareCandidate = caseInsensitive ? normalizedCandidate.toLowerCase() : normalizedCandidate;
  const prefix = compareRoot.endsWith("/") ? compareRoot : `${compareRoot}/`;
  return compareCandidate === compareRoot || compareCandidate.startsWith(prefix);
}

async function workspaceFolder(workspaceId: string): Promise<string> {
  const workspaces = await call("workspace_list");
  const workspace = workspaces.find((item) => item.id === workspaceId);
  if (!workspace) throw new Error("workspace not found");
  return lexicalNormalize(workspace.folder);
}

async function scopedPath(workspaceId: string, path: string): Promise<string> {
  const root = await workspaceFolder(workspaceId);
  const candidate = lexicalNormalize(path && path !== "." ? (isAbsolutePath(path) ? path : `${root}/${path}`) : root);
  if (!isWithin(root, candidate)) throw new Error("path escapes workspace");
  return candidate;
}

async function scopedList(workspaceId: string, path: string): Promise<FsEntry[]> {
  const folder = await scopedPath(workspaceId, path);
  const entries = await withTimeout(readDir(folder), SCOPED_TIMEOUT_MS);
  return entries
    .filter((entry) => !entry.isSymlink)
    .map((entry) => ({
      name: entry.name,
      path: `${folder.endsWith("/") ? folder : `${folder}/`}${entry.name}`,
      directory: entry.isDirectory,
    }))
    .sort((a, b) => Number(b.directory) - Number(a.directory) || a.name.localeCompare(b.name));
}

async function scopedRead(workspaceId: string, path: string): Promise<string> {
  return withTimeout(readTextFile(await scopedPath(workspaceId, path)), SCOPED_TIMEOUT_MS);
}

async function scopedWrite(workspaceId: string, path: string, contents: string): Promise<void> {
  await withTimeout(writeTextFile(await scopedPath(workspaceId, path), contents), SCOPED_TIMEOUT_MS);
}

export function listWorkspace(workspaceId: string, path: string): Promise<FsEntry[]> {
  // The native command owns the workspace-root check and does not depend on
  // the WebView scope being granted in the same tick as a cold launch.
  return retry(() => withTimeout(call("fs_list", { workspaceId, path }), PRIMARY_TIMEOUT_MS))
    .catch(() => retry(() => scopedList(workspaceId, path)))
    .catch(() => { throw new Error(ACCESS_TIMEOUT_MESSAGE); });
}

export function readWorkspaceFile(workspaceId: string, path: string): Promise<string> {
  return retry(() => withTimeout(call("fs_read", { workspaceId, path }), PRIMARY_TIMEOUT_MS))
    .catch(() => retry(() => scopedRead(workspaceId, path)))
    .catch(() => { throw new Error(ACCESS_TIMEOUT_MESSAGE); });
}

export function writeWorkspaceFile(workspaceId: string, path: string, contents: string): Promise<void> {
  return retry(() => withTimeout(call("fs_write", { workspaceId, path, contents }), PRIMARY_TIMEOUT_MS))
    .catch(() => retry(() => scopedWrite(workspaceId, path, contents)))
    .catch(() => { throw new Error(ACCESS_TIMEOUT_MESSAGE); });
}
