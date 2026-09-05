import type { PaneLayout } from "@harbor/schema/commands";

/** Normalize split ratios to 1/n along each split. No new nodes. */
export function tidy(layout: PaneLayout): PaneLayout {
  switch (layout.type) {
    case "leaf":
      return layout;
    case "tabs":
      return layout;
    case "split":
      return {
        type: "split",
        dir: layout.dir,
        ratio: 0.5,
        a: tidy(layout.a),
        b: tidy(layout.b),
      };
  }
}
