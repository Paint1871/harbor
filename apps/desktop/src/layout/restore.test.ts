import { describe, expect, it } from "vitest";
import type { PaneLayout } from "@harbor/schema/commands";
import { dropLeaf, leafPaneIds, normalizeLayout, preferredSplitDir, splitLeaf } from "./restore";

describe("restored pane layouts", () => {
  it("removes duplicate references while preserving the first pane", () => {
    const layout: PaneLayout = {
      type: "split",
      dir: "h",
      ratio: 0.5,
      a: { type: "leaf", paneId: "terminal" },
      b: {
        type: "split",
        dir: "v",
        ratio: 0.5,
        a: { type: "leaf", paneId: "terminal" },
        b: { type: "leaf", paneId: "files" },
      },
    };

    const normalized = normalizeLayout(layout);

    expect(leafPaneIds(normalized)).toEqual(["terminal", "files"]);
    expect(normalized).toEqual({
      type: "split",
      dir: "h",
      ratio: 0.5,
      a: { type: "leaf", paneId: "terminal" },
      b: { type: "leaf", paneId: "files" },
    });
  });

  it("splits only the first matching leaf", () => {
    const layout: PaneLayout = {
      type: "split",
      dir: "h",
      ratio: 0.5,
      a: { type: "leaf", paneId: "terminal" },
      b: { type: "leaf", paneId: "terminal" },
    };

    const split = splitLeaf(layout, "terminal", "new-terminal");

    expect(leafPaneIds(split)).toEqual(["terminal", "new-terminal"]);
    expect(JSON.stringify(split).match(/new-terminal/g)).toHaveLength(1);
  });

  it("closes one matching leaf without touching another branch", () => {
    const layout: PaneLayout = {
      type: "split",
      dir: "h",
      ratio: 0.5,
      a: { type: "leaf", paneId: "terminal" },
      b: { type: "leaf", paneId: "terminal" },
    };

    expect(dropLeaf(layout, "terminal")).toEqual({ type: "leaf", paneId: "terminal" });
  });
});

describe("preferredSplitDir", () => {
  it("splits a wide pane into columns and a tall pane into rows", () => {
    const single: PaneLayout = { type: "leaf", paneId: "a" };
    expect(preferredSplitDir(single, "a", 1200, 700)).toBe("h");
    expect(preferredSplitDir(single, "a", 500, 900)).toBe("v");
  });

  it("uses the pane's own box, not the whole grid", () => {
    // Two columns in a wide grid: each half is now taller than it is wide.
    const columns: PaneLayout = {
      type: "split",
      dir: "h",
      ratio: 0.5,
      a: { type: "leaf", paneId: "a" },
      b: { type: "leaf", paneId: "b" },
    };
    expect(preferredSplitDir(columns, "a", 1000, 800)).toBe("v");
    expect(preferredSplitDir(columns, "b", 1000, 800)).toBe("v");
    // A stack of rows in the same grid: each row is wide and short.
    const rows: PaneLayout = { ...columns, dir: "v" };
    expect(preferredSplitDir(rows, "a", 1000, 800)).toBe("h");
  });

  it("follows an uneven ratio", () => {
    const columns: PaneLayout = {
      type: "split",
      dir: "h",
      ratio: 0.8,
      a: { type: "leaf", paneId: "wide" },
      b: { type: "leaf", paneId: "narrow" },
    };
    expect(preferredSplitDir(columns, "wide", 1000, 600)).toBe("h");
    expect(preferredSplitDir(columns, "narrow", 1000, 600)).toBe("v");
  });

  it("splits a new pane along the long axis so halves stay usable", () => {
    const start: PaneLayout = { type: "leaf", paneId: "a" };
    const wide = splitLeaf(start, "a", "b", preferredSplitDir(start, "a", 1200, 900));
    expect(wide).toMatchObject({ type: "split", dir: "h" });
    // Each column is now 600x900, so the next split stacks instead of slicing.
    expect(preferredSplitDir(wide, "b", 1200, 900)).toBe("v");
    // A square pane is a tie; columns win, which matches a wide window.
    expect(preferredSplitDir(wide, "b", 1200, 600)).toBe("h");
  });
});
