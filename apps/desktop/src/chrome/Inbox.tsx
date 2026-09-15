import { useEffect, useState } from "react";
import { call } from "../ipc";
import { listen } from "@tauri-apps/api/event";
import { Card } from "@harbor/ui/Card";
import type { Notification } from "@harbor/schema/commands";
import { useOptionalChrome, type OpenSessionTarget } from "./chrome-context";

interface InboxProps {
  open: boolean;
  onClose?: () => void;
}

const KIND_LABEL: Record<string, string> = {
  permission: "Needs you",
  mail: "Mail",
  "terminal-exit": "Terminal",
  "turn-finished": "Finished",
  "turn-cancelled": "Cancelled",
  "turn-refused": "Refused",
  "turn-stopped": "Stopped",
};

const KIND_FILTERS = [
  { id: "all", label: "All" },
  { id: "permission", label: KIND_LABEL.permission },
  { id: "mail", label: KIND_LABEL.mail },
  { id: "terminal-exit", label: KIND_LABEL["terminal-exit"] },
  { id: "turn-finished", label: KIND_LABEL["turn-finished"] },
] as const;

type KindFilter = (typeof KIND_FILTERS)[number]["id"];

function asMode(value: string | null): OpenSessionTarget["mode"] | null {
  if (value === "agent" || value === "chat" || value === "code") return value;
  return null;
}

function relativeTime(seconds: number): string {
  const delta = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return `${Math.floor(delta / 86400)}d ago`;
}

/** Events Harbor recorded while you were somewhere else. */
export function Inbox({ open, onClose }: InboxProps) {
  const chrome = useOptionalChrome();
  const [events, setEvents] = useState<Notification[]>([]);
  const [failed, setFailed] = useState(false);
  const [kindFilter, setKindFilter] = useState<KindFilter>("all");

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      void call("notifications_list")
        .then((rows) => {
          if (cancelled) return;
          setEvents(Array.isArray(rows) ? rows : []);
          setFailed(false);
        })
        .catch(() => {
          if (!cancelled) setFailed(true);
        });
    };
    load();
    // Keep an open panel current while work continues elsewhere.
    const stop = listen("notification", load).catch(() => undefined);
    return () => {
      cancelled = true;
      void stop.then((off) => (typeof off === "function" ? off() : undefined));
    };
  }, []);

  useEffect(() => {
    // Opening the panel is what counts as reading it.
    if (open) void call("notifications_mark_read").catch(() => undefined);
  }, [open, events.length]);

  function clearInbox() {
    void call("notifications_clear")
      .then(() => setEvents([]))
      .catch(() => undefined);
  }

  function go(event: Notification) {
    if (!chrome) return;
    const mode = asMode(event.mode);
    const hasSession = Boolean(event.sessionRef);
    const hasPane = Boolean(event.workspaceId && event.paneId);
    if (mode && (hasSession || hasPane) && chrome.onOpenSession) {
      chrome.onOpenSession({
        mode,
        sessionRef: event.sessionRef,
        workspaceId: event.workspaceId,
        paneId: event.paneId,
      });
    } else if (event.mode === "code" && event.workspaceId && event.paneId) {
      chrome.onCodePaneSelect?.(event.workspaceId, event.paneId);
    } else if (mode) {
      chrome.onModeChange(mode);
    }
    onClose?.();
  }

  const visible = kindFilter === "all" ? events : events.filter((event) => event.kind === kindFilter);

  return (
    <Card className="harbor-inbox" aria-label="Notifications" data-open={open} inert={!open}>
      <header>
        <div className="harbor-inbox-heading">
          <h2>Inbox</h2>
          <span className="harbor-eyebrow">Events</span>
          <button
            type="button"
            className="harbor-inbox-clear"
            aria-label="Clear inbox"
            disabled={events.length === 0}
            hidden={events.length === 0}
            onClick={clearInbox}
          >
            Clear
          </button>
        </div>
        <div className="harbor-inbox-filters" role="group" aria-label="Filter events">
          {KIND_FILTERS.map((filter) => (
            <button
              type="button"
              key={filter.id}
              aria-pressed={kindFilter === filter.id}
              onClick={() => setKindFilter(filter.id)}
            >
              {filter.label}
            </button>
          ))}
        </div>
      </header>
      {failed ? (
        <p className="harbor-inbox-empty">Harbor could not read the local database.</p>
      ) : events.length === 0 ? (
        <p className="harbor-inbox-empty">
          Nothing yet. Finished turns, terminals that stop, and anything waiting on you land here.
        </p>
      ) : visible.length === 0 ? (
        <p className="harbor-inbox-empty">No matching events.</p>
      ) : (
        <ul>
          {visible.map((event) => (
            <li key={event.id} data-unread={!event.read} data-kind={event.kind}>
              <button type="button" onClick={() => go(event)}>
                <span className="harbor-inbox-kind">{KIND_LABEL[event.kind] ?? event.kind}</span>
                <span className="harbor-inbox-copy">
                  <strong>{event.title}</strong>
                  {event.body ? <span>{event.body}</span> : null}
                </span>
                <span className="harbor-inbox-time">{relativeTime(event.createdAt)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
