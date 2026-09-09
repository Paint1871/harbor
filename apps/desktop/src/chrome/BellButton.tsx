import { useEffect, useState } from "react";
import { call } from "../ipc";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@harbor/ui/Button";

interface BellButtonProps {
  onClick: () => void;
  /** Rises while the inbox is closed; the panel clears it on open. */
  suppressed?: boolean;
}

export function BellButton({ onClick, suppressed = false }: BellButtonProps) {
  const [unread, setUnread] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const read = () => {
      void call("notifications_unread_count")
        .then((count) => {
          if (!cancelled) setUnread(Number.isFinite(count) ? count : 0);
        })
        .catch(() => undefined);
    };
    read();
    // The host pushes each new row, so the count never waits on a poll.
    const stop = listen("notification", read).catch(() => undefined);
    return () => {
      cancelled = true;
      void stop.then((off) => (typeof off === "function" ? off() : undefined));
    };
  }, []);

  useEffect(() => {
    if (suppressed) setUnread(0);
  }, [suppressed]);

  const label = unread > 0 ? `Notifications, ${unread} unread` : "Notifications";
  return (
    <Button size="icon" variant="ghost" aria-label={label} onClick={onClick}>
      <span className="harbor-bell">
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.75c-2.2 0-4 1.7-4 3.9v1.7c0 .7-.3 1.4-.8 1.9L2.4 10.2c-.3.3-.1.8.3.8h10.6c.4 0 .6-.5.3-.8l-.8-.95A2.7 2.7 0 0 1 12 7.35V5.65c0-2.2-1.8-3.9-4-3.9Z"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.4"
            strokeLinejoin="round"
          />
          <path d="M6.4 12.4a1.7 1.7 0 0 0 3.2 0" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
        </svg>
        {unread > 0 ? (
          <span className="harbor-bell-badge" aria-hidden="true">
            {unread > 9 ? "9+" : unread}
          </span>
        ) : null}
      </span>
    </Button>
  );
}
