import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "@harbor/ui/Card";
import type { Notification } from "@harbor/schema/commands";

interface InboxProps {
  open: boolean;
}

/** Shows the local notifications table. 0.1.0 writes rows for agent mail. */
export function Inbox({ open }: InboxProps) {
  const [events, setEvents] = useState<Notification[]>([]);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void invoke<Notification[]>("notifications_list")
      .then((rows) => {
        if (cancelled) return;
        setEvents(rows);
        setFailed(false);
        return invoke("notifications_mark_read");
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  if (!open) return null;
  return (
    <Card className="harbor-inbox" aria-label="Notifications">
      <header>
        <h2>Inbox</h2>
        <span className="harbor-eyebrow">Events</span>
      </header>
      {failed ? (
        <p className="harbor-inbox-empty">Harbor could not read the local database.</p>
      ) : events.length === 0 ? (
        <p className="harbor-inbox-empty">Nothing yet. Agent events land here.</p>
      ) : (
        <ul>
          {events.map((event) => (
            <li key={event.id} data-unread={!event.read}>
              <span className="harbor-inbox-kind">{event.kind}</span>
              <span className="harbor-inbox-copy">
                {event.title}
                {event.body ? ` — ${event.body}` : ""}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}
