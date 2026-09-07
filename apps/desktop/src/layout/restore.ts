import type { PaneLayout } from "@harbor/schema/commands";

export function restoredLayoutPaused(layout: PaneLayout): PaneLayout {
  return normalizeLayout(layout);
}

export function leafPaneIds(layout: PaneLayout): string[] {
  const ids = new Set<string>();
  const collect = (node: PaneLayout) => {
    switch (node.type) {
      case "leaf":
        ids.add(node.paneId);
        break;
      case "split":
        collect(node.a);
        collect(node.b);
        break;
      case "tabs":
        node.kids.forEach((id) => ids.add(id));
        break;
    }
  };
  collect(layout);
  return [...ids];
}

/** Remove stale duplicate references from a restored layout without inventing panes. */
export function normalizeLayout(layout: PaneLayout): PaneLayout {
  const seen = new Set<string>();

  function visit(node: PaneLayout): PaneLayout | null {
    switch (node.type) {
      case "leaf":
        if (seen.has(node.paneId)) return null;
        seen.add(node.paneId);
        return node;
      case "split": {
        const a = visit(node.a);
        const b = visit(node.b);
        if (!a) return b;
        if (!b) return a;
        return { ...node, a, b };
      }
      case "tabs": {
        const kids = node.kids.filter((id) => {
          if (seen.has(id)) return false;
          seen.add(id);
          return true;
        });
        if (!kids.length) return null;
        const activeId = node.kids[node.active];
        const active = activeId ? Math.max(0, kids.indexOf(activeId)) : 0;
        return { type: "tabs", active, kids };
      }
    }
  }

  return visit(layout) ?? { type: "tabs", active: 0, kids: [] };
}

function containsPane(layout: PaneLayout, paneId: string): boolean {
  return leafPaneIds(layout).includes(paneId);
}

export function dropLeaf(layout: PaneLayout, paneId: string): PaneLayout | null {
  switch (layout.type) {
    case "leaf":
      return layout.paneId === paneId ? null : layout;
    case "split": {
      // Pane ids are unique in a valid layout. If an older saved layout is
      // malformed, only touch the first matching branch so one click cannot
      // close several visible panes at once.
      if (containsPane(layout.a, paneId)) {
        const a = dropLeaf(layout.a, paneId);
        if (!a) return layout.b;
        return { ...layout, a };
      }
      if (containsPane(layout.b, paneId)) {
        const b = dropLeaf(layout.b, paneId);
        if (!b) return layout.a;
        return { ...layout, b };
      }
      return layout;
    }
    case "tabs": {
      const index = layout.kids.indexOf(paneId);
      if (index < 0) return layout;
      const kids = layout.kids.filter((_id, childIndex) => childIndex !== index);
      if (!kids.length) return null;
      return { type: "tabs", active: Math.min(layout.active, kids.length - 1), kids };
    }
  }
}

/**
 * Split `paneId` in two. `dir` follows the pane's own shape at the call site:
 * splitting a tall pane into columns (or a wide one into rows) is what made new
 * panes look wedged in.
 */
export function splitLeaf(
  layout: PaneLayout,
  paneId: string,
  newPaneId: string,
  dir: "h" | "v" = "h",
): PaneLayout {
  switch (layout.type) {
    case "leaf":
      if (layout.paneId !== paneId) return layout;
      return {
        type: "split",
        dir,
        ratio: 0.5,
        a: layout,
        b: { type: "leaf", paneId: newPaneId },
      };
    case "split":
      if (containsPane(layout.a, paneId)) {
        return { ...layout, a: splitLeaf(layout.a, paneId, newPaneId, dir) };
      }
      if (containsPane(layout.b, paneId)) {
        return { ...layout, b: splitLeaf(layout.b, paneId, newPaneId, dir) };
      }
      return layout;
    case "tabs":
      return layout;
  }
}

/**
 * Which way a new pane should divide `paneId`.
 *
 * Walks the tree to work out the pane's own box, then splits its long axis. A
 * fixed direction is what wedged new panes into slivers: splitting an already
 * narrow column into two narrower columns.
 */
export function preferredSplitDir(
  layout: PaneLayout,
  paneId: string,
  width: number,
  height: number,
): "h" | "v" {
  const box = leafBox(layout, paneId, width, height);
  if (!box) return width >= height ? "h" : "v";
  return box.width >= box.height ? "h" : "v";
}

function leafBox(
  node: PaneLayout,
  paneId: string,
  width: number,
  height: number,
): { width: number; height: number } | null {
  switch (node.type) {
    case "leaf":
      return node.paneId === paneId ? { width, height } : null;
    case "tabs":
      // Tabs stack in place, so every child fills the same box.
      return node.kids.includes(paneId) ? { width, height } : null;
    case "split": {
      const horizontal = node.dir !== "v";
      const ratio = Math.min(0.95, Math.max(0.05, node.ratio));
      if (horizontal) {
        const first = width * ratio;
        return (
          leafBox(node.a, paneId, first, height)
          ?? leafBox(node.b, paneId, width - first, height)
        );
      }
      const first = height * ratio;
      return (
        leafBox(node.a, paneId, width, first)
        ?? leafBox(node.b, paneId, width, height - first)
      );
    }
  }
}
