export type PaneIconKind = "terminal" | "agent" | "files" | "browser" | "thread";

const STROKE = { fill: "none", stroke: "currentColor", strokeWidth: 1.3, strokeLinecap: "round", strokeLinejoin: "round" } as const;

/**
 * Rail and pane icons on one 16px grid with one stroke weight. Text glyphs used
 * to stand in here; their per-font metrics forced size and colour overrides and
 * never lined up with each other.
 */
export function PaneIcon({ kind }: { kind: PaneIconKind }) {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      {kind === "terminal" ? (
        <>
          <rect x="1.75" y="3" width="12.5" height="10" rx="2" {...STROKE} />
          <path d="M4.75 6.6 6.9 8.4 4.75 10.2M8.9 10.4h2.6" {...STROKE} />
        </>
      ) : null}
      {kind === "agent" ? (
        <>
          <circle cx="8" cy="8" r="2.1" {...STROKE} />
          <path d="M8 1.9v2.4M8 11.7v2.4M2.7 8h2.4M10.9 8h2.4M4.25 4.25 5.9 5.9M10.1 10.1l1.65 1.65M11.75 4.25 10.1 5.9M5.9 10.1l-1.65 1.65" {...STROKE} />
        </>
      ) : null}
      {kind === "files" ? (
        <path d="M2.5 4.1c0-.75.6-1.35 1.35-1.35h2.2l1.3 1.5h4.8c.75 0 1.35.6 1.35 1.35v6.05c0 .75-.6 1.35-1.35 1.35H3.85c-.75 0-1.35-.6-1.35-1.35Z" {...STROKE} />
      ) : null}
      {kind === "browser" ? (
        <>
          <rect x="1.75" y="3" width="12.5" height="10" rx="2" {...STROKE} />
          <path d="M1.75 6.1h12.5" {...STROKE} />
          <circle cx="4.1" cy="4.55" r=".62" fill="currentColor" />
        </>
      ) : null}
      {kind === "thread" ? (
        <path d="M13.4 9.15c0 .83-.67 1.5-1.5 1.5H6.55L3.3 13.1V4.35c0-.83.67-1.5 1.5-1.5h7.1c.83 0 1.5.67 1.5 1.5Z" {...STROKE} />
      ) : null}
    </svg>
  );
}

/** Points down when the row is open, right when it is closed. */
export function DisclosureIcon({ open }: { open: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
      style={{ transform: open ? "rotate(90deg)" : undefined, transition: "transform 120ms ease" }}
    >
      <path d="m6.25 3.75 4.25 4.25-4.25 4.25" {...STROKE} />
    </svg>
  );
}
