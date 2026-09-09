import { useState } from "react";
import { RailRow } from "@harbor/ui/RailRow";
import { Pill } from "@harbor/ui/Pill";
import type { ThreadRecord } from "@harbor/schema/commands";

interface ThreadListProps {
  threads: ThreadRecord[];
  activeId: string | null;
  onSelect: (thread: ThreadRecord) => void;
  onPin: (id: string, pinned: boolean) => void;
  onRename: (id: string, title: string) => void;
  onDelete: (id: string) => void;
}

/**
 * `RailRow` is a button, so rename replaces the row rather than nesting a field
 * inside it. Delete asks once in place: a thread carries its whole transcript.
 */
export function ThreadList({ threads, activeId, onSelect, onPin, onRename, onDelete }: ThreadListProps) {
  const [renaming, setRenaming] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);

  return <div className="harbor-thread-list">
    {threads.map((thread) => <div className="harbor-thread-row" key={thread.id}>
      {renaming === thread.id
        ? <input className="harbor-chat-tab-rename" aria-label={`Rename ${thread.title}`} autoFocus defaultValue={thread.title} maxLength={60}
            onBlur={(event) => {
              const title = event.target.value.trim();
              setRenaming(null);
              if (title && title !== thread.title) onRename(thread.id, title);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") event.currentTarget.blur();
              if (event.key === "Escape") {
                event.currentTarget.value = thread.title;
                event.currentTarget.blur();
              }
            }} />
        : <>
            <RailRow label={thread.title} description={thread.engineId} selected={activeId === thread.id}
              trailing={thread.unread ? <Pill tone="attention">Unread</Pill> : null} onClick={() => onSelect(thread)}
              onDoubleClick={() => { setConfirming(null); setRenaming(thread.id); }} />
            <span className="harbor-chat-tab-actions">
              <button type="button" aria-label={`Rename ${thread.title}`} title="Rename"
                onClick={() => { setConfirming(null); setRenaming(thread.id); }}>✎</button>
              <button type="button" aria-label={`Delete ${thread.title}`} title="Delete" data-confirm={confirming === thread.id}
                onClick={() => {
                  if (confirming !== thread.id) { setConfirming(thread.id); return; }
                  setConfirming(null);
                  onDelete(thread.id);
                }}>{confirming === thread.id ? "Delete?" : "✕"}</button>
            </span>
            <button type="button" className="harbor-pin" aria-label={`${thread.pinned ? "Unpin" : "Pin"} ${thread.title}`}
              aria-pressed={thread.pinned} onClick={() => onPin(thread.id, !thread.pinned)}>{thread.pinned ? "Unpin" : "Pin"}</button>
          </>}
    </div>)}
  </div>;
}
