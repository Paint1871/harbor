import { useEffect, useState } from "react";
import { call } from "../../ipc";
import { RailRow } from "@harbor/ui/RailRow";
import { Pill } from "@harbor/ui/Pill";
import type { ThreadRecord } from "@harbor/schema/commands";

interface ThreadListProps {
  threads: ThreadRecord[];
  /** `null` lists threads that belong to no folder; the search scope follows it. */
  workspaceId?: string | null;
  activeId: string | null;
  onSelect: (thread: ThreadRecord) => void;
  onPin: (id: string, pinned: boolean) => void;
  onRename: (id: string, title: string) => void;
  onDelete: (id: string) => void;
}

export function threadVisible(thread: { title: string; engineId: string; unread: boolean }, query: string, unreadOnly: boolean): boolean {
  if (unreadOnly && !thread.unread) return false;
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return thread.title.toLowerCase().includes(needle) || thread.engineId.toLowerCase().includes(needle);
}

/**
 * `RailRow` is a button, so rename replaces the row rather than nesting a field
 * inside it. Delete asks once in place: a thread carries its whole transcript.
 * Search also matches message contents via `session_search`, not only titles.
 */
export function ThreadList({ threads, workspaceId = null, activeId, onSelect, onPin, onRename, onDelete }: ThreadListProps) {
  const [renaming, setRenaming] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [contentHits, setContentHits] = useState<Set<string>>(new Set());
  const needle = query.trim();
  const visible = threads.filter((thread) => {
    if (unreadOnly && !thread.unread) return false;
    return threadVisible(thread, query, false) || contentHits.has(thread.id);
  });

  useEffect(() => {
    const needle = query.trim();
    if (needle.length < 2) {
      setContentHits(new Set());
      return;
    }
    let active = true;
    const timer = window.setTimeout(() => {
      void call("session_search", { workspaceId: workspaceId ?? "", query: needle })
        .then((hits) => { if (active) setContentHits(new Set(hits.map((hit) => hit.chatId))); })
        .catch(() => { if (active) setContentHits(new Set()); });
    }, 250);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [query, workspaceId]);

  return <div className="harbor-thread-list">
    <div className="harbor-thread-filters">
      <label className="harbor-destination-search harbor-thread-search">Search threads<input aria-label="Search threads" value={query} placeholder="Find a thread" onChange={(event) => setQuery(event.target.value)} /></label>
      <button type="button" className="harbor-thread-unread" aria-pressed={unreadOnly} onClick={() => setUnreadOnly((on) => !on)}>Unread</button>
    </div>
    {visible.length ? visible.map((thread) => <div className="harbor-thread-row" key={thread.id}>
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
    </div>) : unreadOnly
      ? <p className="harbor-muted">{needle ? `No unread threads match “${query}”.` : "No unread threads."}</p>
      : needle ? <p className="harbor-muted">No threads match “{query}”.</p> : null}
  </div>;
}
