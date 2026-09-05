import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FsEntry } from "@harbor/schema/commands";

interface TreeProps {
  workspaceId?: string;
  onOpen: (path: string) => void;
  path?: string;
}

export function Tree({ workspaceId, onOpen, path = "" }: TreeProps) {
  const [entries, setEntries] = useState<FsEntry[]>([]);
  const [openDirs, setOpenDirs] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (!workspaceId) return;
    void invoke<FsEntry[]>("fs_list", { workspaceId, path })
      .then(setEntries)
      .catch(() => setEntries([]));
  }, [workspaceId, path]);

  if (!workspaceId) return <p className="harbor-muted">Add a workspace to browse files.</p>;
  return (
    <ul className="harbor-tree">
      {entries.map((entry) => (
        <li key={entry.path}>
          <button
            type="button"
            onClick={() => {
              if (entry.directory) {
                setOpenDirs((current) => ({ ...current, [entry.path]: !current[entry.path] }));
                return;
              }
              onOpen(entry.path);
            }}
          >
            {entry.directory ? (openDirs[entry.path] ? "▾ " : "▸ ") : ""}
            {entry.name}
          </button>
          {entry.directory && openDirs[entry.path] ? (
            <Tree workspaceId={workspaceId} onOpen={onOpen} path={entry.path} />
          ) : null}
        </li>
      ))}
    </ul>
  );
}
