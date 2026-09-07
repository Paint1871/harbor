import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Card } from "@harbor/ui/Card";
import type { Notification } from "@harbor/schema/commands";
import { useOptionalChrome } from "./chrome-context";

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

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      void invoke<Notification[]>("notifications_list")
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
    if (open) void invoke("notifications_mark_read").catch(() => undefined);
  }, [open, events.length]);

  function go(event: Notification) {
    if (!chrome) return;
    if (event.mode === "code" && event.workspaceId && event.paneId) {
      chrome.onCodePaneSelect?.(event.workspaceId, event.paneId);
    } else if (event.mode === "agent" || event.mode === "code" || event.mode === "chat") {
      chrome.onModeChange(event.mode);
    }
    onClose?.();
  }

  return (
    <Card className="harbor-inbox" aria-label="Notifications" data-open={open} inert={!open}>
      <header>
        <h2>Inbox</h2>
        <span className="harbor-eyebrow">Events</span>
      </header>
      {failed ? (
        <p className="harbor-inbox-empty">Harbor could not read the local database.</p>
      ) : events.length === 0 ? (
        <p className="harbor-inbox-empty">
          Nothing yet. Finished turns, terminals that stop, and anything waiting on you land here.
        </p>
      ) : (
        <ul>
          {events.map((event) => (
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
