import { useEffect, useRef, type FormEvent } from "react";
import { PaneHeader } from "./PaneHeader";

const DEFAULT_URL = "http://localhost:3000/pricing";

function displayUrl(value: string): string {
  return value.replace(/^https?:\/\//i, "");
}

export interface BrowserPaneState {
  draft: string;
  url: string;
  history: string[];
  historyIndex: number;
  frameKey: number;
  status: "loading" | "ready" | "offline";
}

export function createBrowserPaneState(): BrowserPaneState {
  return {
    draft: displayUrl(DEFAULT_URL),
    url: DEFAULT_URL,
    history: [DEFAULT_URL],
    historyIndex: 0,
    frameKey: 0,
    status: "loading",
  };
}

interface BrowserPaneProps {
  state: BrowserPaneState;
  onStateChange: (update: (current: BrowserPaneState) => BrowserPaneState) => void;
  focused: boolean;
  expanded?: boolean;
  onFocus: () => void;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

function normalizeUrl(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return DEFAULT_URL;
  return /^https?:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`;
}

export function BrowserPane({ state, onStateChange, focused, expanded = false, onFocus, onExpand, onSplit, onClose }: BrowserPaneProps) {
  const { draft, url, history, historyIndex, frameKey, status } = state;
  const onStateChangeRef = useRef(onStateChange);
  onStateChangeRef.current = onStateChange;

  useEffect(() => {
    onStateChangeRef.current((current) => ({ ...current, status: "loading" }));
    const timeout = window.setTimeout(() => onStateChangeRef.current((current) => current.status === "loading" ? { ...current, status: "offline" } : current), 2600);
    return () => window.clearTimeout(timeout);
  }, [frameKey, url]);

  function navigate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextUrl = normalizeUrl(draft);
    onStateChange((current) => ({
      ...current,
      draft: displayUrl(nextUrl),
      url: nextUrl,
      history: [...current.history.slice(0, current.historyIndex + 1), nextUrl],
      historyIndex: current.historyIndex + 1,
      status: "loading",
    }));
  }

  function moveHistory(direction: -1 | 1) {
    const nextIndex = historyIndex + direction;
    if (nextIndex < 0 || nextIndex >= history.length) return;
    const nextUrl = history[nextIndex];
    if (!nextUrl) return;
    onStateChange((current) => ({ ...current, historyIndex: nextIndex, url: nextUrl, draft: displayUrl(nextUrl), status: "loading" }));
  }

  return (
    <section className="harbor-pane harbor-browser-pane" data-focused={focused} onClick={onFocus} aria-label="localhost:3000">
      <PaneHeader
        title="localhost:3000"
        compact
        extra={
          <div className="harbor-browser-toolbar-inline">
            <button type="button" aria-label="Back" title="Back" disabled={historyIndex === 0} onClick={() => moveHistory(-1)}>‹</button>
            <button type="button" aria-label="Forward" title="Forward" disabled={historyIndex >= history.length - 1} onClick={() => moveHistory(1)}>›</button>
            <button type="button" aria-label="Reload preview" title="Reload preview" onClick={() => onStateChange((current) => ({ ...current, frameKey: current.frameKey + 1, status: "loading" }))}>↻</button>
            <form onSubmit={navigate}>
              <input aria-label="Preview URL" value={draft} onChange={(event) => onStateChange((current) => ({ ...current, draft: event.target.value }))} />
            </form>
            <span className="harbor-browser-status harbor-sr-only" data-status={status} role="status">{status === "ready" ? "Live" : status === "offline" ? "Offline" : "Loading"}</span>
          </div>
        }
        expanded={expanded}
        onExpand={onExpand}
        onSplit={onSplit}
        onClose={onClose}
      />
      <div className="harbor-browser-surface">
        <iframe
          key={`${url}:${frameKey}`}
          title="Preview frame"
          src={url}
          sandbox="allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
          onLoad={() => onStateChange((current) => ({ ...current, status: "ready" }))}
          aria-hidden={status !== "ready"}
        />
        {status !== "ready" ? (
          <div className="harbor-browser-placeholder" role="status">
            <div className="harbor-browser-skeleton harbor-browser-skeleton-wide" />
            <div className="harbor-browser-skeleton harbor-browser-skeleton-medium" />
            <div className="harbor-browser-cards">
              <span />
              <span data-active="true" />
              <span />
            </div>
            <div className="harbor-browser-skeleton harbor-browser-skeleton-footer" />
          </div>
        ) : null}
      </div>
    </section>
  );
}
