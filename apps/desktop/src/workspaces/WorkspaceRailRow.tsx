import { useEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
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
}

/**
 * A workspace row that can be renamed in place. `RailRow` is a button, so the
 * editor replaces the row rather than nesting a control inside it. Rename opens
 * on double click or F2; Escape cancels and Enter commits.
 */
export function WorkspaceRailRow({
  workspace,
  selected = false,
  className,
  leading,
  description,
  onSelect,
  onRenamed,
}: WorkspaceRailRowProps) {
  const name = workspaceName(workspace);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) input.current?.select();
  }, [editing]);

  function open() {
    setDraft(name);
    setError(null);
    setEditing(true);
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
      const updated = await invoke<Workspace>("workspace_rename", { id: workspace.id, title: next });
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
      <div className="harbor-workspace-rename">
        <input
          ref={input}
          value={draft}
          maxLength={60}
          disabled={busy}
          aria-label={`Rename ${name}`}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => void commit()}
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
      </div>
    );
  }

  return (
    <RailRow
      className={className}
      label={name}
      description={description}
      title={`${workspace.folder} — double click or F2 to rename`}
      leading={leading}
      selected={selected}
      onClick={onSelect}
      onDoubleClick={open}
      onKeyDown={(event) => {
        if (event.key === "F2") {
          event.preventDefault();
          open();
        }
      }}
    />
  );
}
