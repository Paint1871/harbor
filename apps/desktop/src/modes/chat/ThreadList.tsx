import { RailRow } from "@harbor/ui/RailRow";
import { Pill } from "@harbor/ui/Pill";
import type { ThreadRecord } from "@harbor/schema/commands";

interface ThreadListProps {
  threads: ThreadRecord[];
  activeId: string | null;
  onSelect: (thread: ThreadRecord) => void;
  onPin: (id: string, pinned: boolean) => void;
}

export function ThreadList({ threads, activeId, onSelect, onPin }: ThreadListProps) {
  return <div className="harbor-thread-list">
    {threads.map((thread) => <div className="harbor-thread-row" key={thread.id}>
      <RailRow label={thread.title} description={thread.engineId} selected={activeId === thread.id}
        trailing={thread.unread ? <Pill tone="attention">Unread</Pill> : null} onClick={() => onSelect(thread)} />
      <button type="button" className="harbor-pin" aria-label={`${thread.pinned ? "Unpin" : "Pin"} ${thread.title}`}
        aria-pressed={thread.pinned} onClick={() => onPin(thread.id, !thread.pinned)}>{thread.pinned ? "Unpin" : "Pin"}</button>
    </div>)}
  </div>;
}
