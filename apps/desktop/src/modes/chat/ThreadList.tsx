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
  return (
    <div className="harbor-thread-list">
      {threads.map((thread) => (
        <RailRow
          key={thread.id}
          label={thread.title}
          description={thread.engineId}
          selected={activeId === thread.id}
          trailing={
            thread.unread ? (
              <Pill tone="attention">Unread</Pill>
            ) : (
              <button type="button" className="harbor-pin" onClick={(event) => { event.stopPropagation(); onPin(thread.id, !thread.pinned); }}>
                {thread.pinned ? "Unpin" : "Pin"}
              </button>
            )
          }
          onClick={() => onSelect(thread)}
        />
      ))}
    </div>
  );
}
