import { writeWorkspaceFile } from "./fs";

/**
 * Central registry of editor buffers with unsaved changes. Pane close, tab
 * close and the window close guard all flush through here so a dirty buffer
 * is never dropped silently just because its component unmounted.
 */
export interface DirtySource {
  workspaceId: string;
  path: string;
  /** True while the buffer holds edits newer than the last successful save. */
  isDirty: () => boolean;
  /** Current buffer contents plus the revision they belong to; null when the editor is gone. */
  read: () => { content: string; revision: number } | null;
  /** Clear the dirty flag only if nothing newer was typed since `revision`. */
  markSaved: (revision: number) => void;
  /** Set when the user chose to discard the buffer; suppresses flush-on-unmount. */
  discarded: boolean;
}

const sources = new Map<string, DirtySource>();

function keyOf(workspaceId: string, path: string): string {
  return `${workspaceId}${path}`;
}

export function registerDirtySource(source: DirtySource): () => void {
  const key = keyOf(source.workspaceId, source.path);
  sources.set(key, source);
  return () => {
    sources.delete(key);
  };
}

/** The user confirmed a discard; the unmount flush must not resurrect the edit. */
export function discardDirtySource(workspaceId: string, path: string): void {
  const source = sources.get(keyOf(workspaceId, path));
  if (source) source.discarded = true;
}

export function hasUnsavedBuffers(): boolean {
  for (const source of sources.values()) {
    if (!source.discarded && source.isDirty()) return true;
  }
  return false;
}

/**
 * Write every dirty buffer, waiting at most `timeoutMs`. Returns the paths
 * that are still dirty afterwards — either the write failed or the buffer
 * changed again while it was in flight.
 */
export async function saveUnsavedBuffers(timeoutMs = 2000): Promise<string[]> {
  const pending = [...sources.values()].filter((source) => !source.discarded && source.isDirty());
  if (!pending.length) return [];
  const timeout = new Promise<"timeout">((resolve) => {
    setTimeout(() => resolve("timeout"), timeoutMs);
  });
  const work = Promise.all(
    pending.map(async (source) => {
      const snapshot = source.read();
      if (snapshot === null) return;
      try {
        await writeWorkspaceFile(source.workspaceId, source.path, snapshot.content);
        source.markSaved(snapshot.revision);
      } catch {
        /* stays dirty and is reported below */
      }
    }),
  ).then(() => "done" as const);
  await Promise.race([work, timeout]);
  return pending.filter((source) => !source.discarded && source.isDirty()).map((source) => source.path);
}
