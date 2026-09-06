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

export function splitLeaf(layout: PaneLayout, paneId: string, newPaneId: string): PaneLayout {
  switch (layout.type) {
    case "leaf":
      if (layout.paneId !== paneId) return layout;
      return {
        type: "split",
        dir: "h",
        ratio: 0.5,
        a: layout,
        b: { type: "leaf", paneId: newPaneId },
      };
    case "split":
      if (containsPane(layout.a, paneId)) {
        return { ...layout, a: splitLeaf(layout.a, paneId, newPaneId) };
      }
      if (containsPane(layout.b, paneId)) {
        return { ...layout, b: splitLeaf(layout.b, paneId, newPaneId) };
      }
      return layout;
    case "tabs":
      return layout;
  }
}
