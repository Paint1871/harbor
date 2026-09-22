import { useState, type FormEvent } from "react";
import { call } from "../ipc";
import { PaneHeader } from "./PaneHeader";

const DEFAULT_URL = "http://localhost:3000";

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
  onFocus?: () => void;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

function normalizeUrl(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return DEFAULT_URL;
  return /^https?:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`;
}

/**
 * An embedded browser is a post-0.1.0 pane (a wry child webview), and the app
 * CSP forbids frames anyway (`frame-src 'none'`). This pane is the address bar
 * plus the hand-off to the system browser — it never pretends to render a page.
 */
export function BrowserPane({ state, onStateChange, focused, expanded = false, onFocus, onExpand, onSplit, onClose }: BrowserPaneProps) {
  const { draft, url, history, historyIndex } = state;
  const [openError, setOpenError] = useState<string | null>(null);

  function navigate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const nextUrl = normalizeUrl(draft);
    onStateChange((current) => ({
      ...current,
      draft: displayUrl(nextUrl),
      url: nextUrl,
      history: [...current.history.slice(0, current.historyIndex + 1), nextUrl],
      historyIndex: current.historyIndex + 1,
    }));
    void openInSystemBrowser(nextUrl);
  }

  function moveHistory(direction: -1 | 1) {
    const nextIndex = historyIndex + direction;
    if (nextIndex < 0 || nextIndex >= history.length) return;
    const nextUrl = history[nextIndex];
    if (!nextUrl) return;
    onStateChange((current) => ({ ...current, historyIndex: nextIndex, url: nextUrl, draft: displayUrl(nextUrl) }));
  }

  function openInSystemBrowser(target = url) {
    setOpenError(null);
    void call("open_external_url", { url: target }).catch(() => {
      setOpenError("Could not open in system browser");
    });
  }

  return (
    <section className="harbor-pane harbor-browser-pane" data-focused={focused} onClick={onFocus} aria-label="Browser preview">
      <PaneHeader
        title="localhost:3000"
        compact
        extra={
          <div className="harbor-browser-toolbar-inline">
            <button type="button" aria-label="Back" title="Back" disabled={historyIndex === 0} onClick={() => moveHistory(-1)}>‹</button>
            <button type="button" aria-label="Forward" title="Forward" disabled={historyIndex >= history.length - 1} onClick={() => moveHistory(1)}>›</button>
            <form onSubmit={navigate}>
              <input aria-label="Preview URL" value={draft} onChange={(event) => onStateChange((current) => ({ ...current, draft: event.target.value }))} />
            </form>
            <button type="button" className="harbor-browser-open" aria-label="Open in system browser" title="Open in system browser" onClick={() => openInSystemBrowser()}>
              Open in system browser
            </button>
            {openError ? <span className="harbor-browser-status" data-status="offline" role="status">{openError}</span> : null}
          </div>
        }
        expanded={expanded}
        onExpand={onExpand}
        onSplit={onSplit}
        onClose={onClose}
      />
      <div className="harbor-browser-surface">
        <div className="harbor-terminal-state" role="status">
          <span className="harbor-terminal-state-mark" aria-hidden="true">↗</span>
          <strong>Previews open in your browser</strong>
          <p>An embedded preview pane arrives after 0.1.0. Enter an address above and Harbor opens it in the system browser.</p>
        </div>
      </div>
    </section>
  );
}
