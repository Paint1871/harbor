import { useEffect, useRef, useState, type ReactNode } from "react";
import { call } from "../ipc";
import { RailRow } from "@harbor/ui/RailRow";
import type { Workspace } from "@harbor/schema/commands";

export function workspaceName(workspace: Workspace): string {
  if (workspace.title) return workspace.title;
  return workspace.folder.split(/[\\/]/).filter(Boolean).pop() ?? "Workspace";
}

interface WorkspaceRailRowProps {
  workspace: Workspace;
  selected?: boolean;
  className?: string;
  leading?: ReactNode;
  description?: ReactNode;
  onSelect: () => void;
  onRenamed: (workspace: Workspace) => void;
  onRemoved?: (id: string) => void;
}

/**
 * A workspace row that can be renamed in place. `RailRow` is a button, so the
 * editor replaces the row rather than nesting a control inside it.
 *
 * Rename opens on right click or F2, never on double click: the row's own click
 * expands and collapses the workspace, and the first click of a double click
 * would toggle it shut on the way to renaming.
 *
 * Remove lives in that same editor. It is the one gesture that takes a folder
 * off the rail, so it asks once and never sits under the pointer by accident.
 */
export function WorkspaceRailRow({
  workspace,
  selected = false,
  className,
  leading,
  description,
  onSelect,
  onRenamed,
  onRemoved,
}: WorkspaceRailRowProps) {
  const name = workspaceName(workspace);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmingRemove, setConfirmingRemove] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const editor = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (editing) input.current?.select();
  }, [editing]);

  function open() {
    setDraft(name);
    setError(null);
    setConfirmingRemove(false);
    setEditing(true);
  }

  async function remove() {
    if (busy || !onRemoved) return;
    if (!confirmingRemove) {
      setConfirmingRemove(true);
      return;
    }
    setBusy(true);
    try {
      await call("workspace_remove", { id: workspace.id });
      setEditing(false);
      setError(null);
      onRemoved(workspace.id);
    } catch (reason) {
      setError(String(reason));
      setConfirmingRemove(false);
    } finally {
      setBusy(false);
    }
  }

  async function commit() {
    if (busy) return;
    const next = draft.trim();
    if (next === name) {
      setEditing(false);
      return;
    }
    setBusy(true);
    try {
      const updated = await call("workspace_rename", { id: workspace.id, title: next });
      onRenamed(updated);
      setEditing(false);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  if (editing) {
    return (
      <div className="harbor-workspace-rename" ref={editor}>
        <input
          ref={input}
          value={draft}
          maxLength={60}
          disabled={busy}
          aria-label={`Rename ${name}`}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={(event) => {
            // Reaching for Remove must not count as leaving the editor.
            if (editor.current?.contains(event.relatedTarget)) return;
            void commit();
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void commit();
            }
            if (event.key === "Escape") {
              event.preventDefault();
              setError(null);
              setEditing(false);
            }
          }}
        />
        {error ? <p role="alert">{error}</p> : <p>Enter saves · Esc cancels · empty restores the folder name</p>}
        {onRemoved ? (
          <button
            type="button"
            className="harbor-workspace-remove"
            data-confirm={confirmingRemove}
            disabled={busy}
            aria-label={`Remove ${name} from Workspaces`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => void remove()}
          >
            {confirmingRemove ? "Remove? The folder on disk stays." : "Remove from Workspaces"}
          </button>
        ) : null}
      </div>
    );
  }

  return (
    <RailRow
      className={className}
      label={name}
      description={description}
      title={`${workspace.folder} — right click or F2 to rename${onRemoved ? " or remove" : ""}`}
      leading={leading}
      selected={selected}
      onClick={onSelect}
      onContextMenu={(event) => {
        event.preventDefault();
        open();
      }}
      onKeyDown={(event) => {
        if (event.key === "F2") {
          event.preventDefault();
          open();
        }
      }}
    />
  );
}
