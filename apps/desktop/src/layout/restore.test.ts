import { describe, expect, it } from "vitest";
import type { PaneLayout } from "@harbor/schema/commands";
import { dropLeaf, leafPaneIds, normalizeLayout, splitLeaf } from "./restore";

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
