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
      <header>
        <h2>Inbox</h2>
        <span className="harbor-eyebrow">Events</span>
      </header>
      <ul>
        {EVENTS.map((event) => (
          <li key={event.kind}>
            <span className="harbor-inbox-kind">{event.kind}</span>
            <span className="harbor-inbox-copy">{`Local agent ${event.copy}`}</span>
          </li>
        ))}
      </ul>
    </Card>
  );
}
