import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  discardDirtySource,
  hasUnsavedBuffers,
  registerDirtySource,
  saveUnsavedBuffers,
  type DirtySource,
} from "./dirtyFiles";

const writeMock = vi.fn<(workspaceId: string, path: string, contents: string) => Promise<void>>();

vi.mock("./fs", () => ({
  writeWorkspaceFile: (workspaceId: string, path: string, contents: string) =>
    writeMock(workspaceId, path, contents),
}));

function buffer(workspaceId: string, path: string, content: string): DirtySource & { dirty: boolean } {
  const source = {
    workspaceId,
    path,
    discarded: false,
    dirty: true,
    isDirty: () => source.dirty,
    read: () => ({ content, revision: 1 }),
    markSaved: (revision: number) => {
      if (revision === 1) source.dirty = false;
    },
  };
  return source;
}

beforeEach(() => {
  writeMock.mockReset().mockResolvedValue(undefined);
});

describe("dirty file registry", () => {
  it("reports and flushes a registered dirty buffer", async () => {
    const source = buffer("ws-1", "a.ts", "new content");
    const unregister = registerDirtySource(source);
    expect(hasUnsavedBuffers()).toBe(true);

    const failed = await saveUnsavedBuffers();
    expect(failed).toEqual([]);
    expect(writeMock).toHaveBeenCalledWith("ws-1", "a.ts", "new content");
    expect(hasUnsavedBuffers()).toBe(false);
    unregister();
  });

  it("keeps a buffer dirty when the write fails", async () => {
    writeMock.mockRejectedValue(new Error("disk full"));
    const source = buffer("ws-1", "b.ts", "unsaved");
    const unregister = registerDirtySource(source);

    const failed = await saveUnsavedBuffers();
    expect(failed).toEqual(["b.ts"]);
    expect(hasUnsavedBuffers()).toBe(true);
    unregister();
  });

  it("keeps a buffer dirty when it changed again mid-save", async () => {
    const source = buffer("ws-1", "c.ts", "v1");
    // Simulate a second edit arriving while the write is in flight: the
    // revision moves forward, so markSaved must not clear the dirty flag.
    source.markSaved = (revision: number) => {
      if (revision === 2) source.dirty = false;
    };
    const unregister = registerDirtySource(source);

    const failed = await saveUnsavedBuffers();
    expect(failed).toEqual(["c.ts"]);
    unregister();
  });

  it("ignores buffers the user chose to discard", async () => {
    const source = buffer("ws-1", "d.ts", "dropped");
    const unregister = registerDirtySource(source);
    discardDirtySource("ws-1", "d.ts");

    expect(hasUnsavedBuffers()).toBe(false);
    const failed = await saveUnsavedBuffers();
    expect(failed).toEqual([]);
    expect(writeMock).not.toHaveBeenCalled();
    unregister();
  });

  it("unregisters cleanly so saved buffers do not linger", async () => {
    const source = buffer("ws-1", "e.ts", "data");
    const unregister = registerDirtySource(source);
    unregister();
    expect(hasUnsavedBuffers()).toBe(false);
  });
});
