import { Card } from "@harbor/ui/Card";

const EVENTS = [
  { kind: "prompt", copy: "is waiting on a prompt" },
  { kind: "permission", copy: "needs permission" },
  { kind: "question", copy: "asked a question" },
  { kind: "complete", copy: "finished" },
  { kind: "failure", copy: "failed" },
] as const;

interface InboxProps {
  open: boolean;
}

export function Inbox({ open }: InboxProps) {
  if (!open) return null;
  return (
    <Card className="harbor-inbox" aria-label="Notifications">
      <h2>Inbox</h2>
      <p className="harbor-muted">Event types only — no transcript dumps.</p>
      <ul>
        {EVENTS.map((event) => (
          <li key={event.kind}>
            <strong>{event.kind}</strong> — {`{agent} ${event.copy}`}
          </li>
        ))}
      </ul>
    </Card>
  );
}
