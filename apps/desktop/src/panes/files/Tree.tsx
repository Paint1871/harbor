import { useEffect, useRef, useState } from "react";
import type { FsEntry } from "@harbor/schema/commands";
import { filesystemError, listWorkspace } from "./fs";

interface TreeProps {
  workspaceId?: string;
  onOpen: (path: string) => void;
  path?: string;
}

export function Tree({ workspaceId, onOpen, path = "" }: TreeProps) {
  const [entries, setEntries] = useState<FsEntry[]>([]);
  const [openDirs, setOpenDirs] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const autoRetryRef = useRef(false);
  const scopeRef = useRef<{ workspaceId?: string; path?: string }>({});

  useEffect(() => {
    if (!workspaceId) {
      setEntries([]);
      setLoading(false);
      return;
    }
    if (scopeRef.current.workspaceId !== workspaceId || scopeRef.current.path !== path) {
      autoRetryRef.current = false;
      scopeRef.current = { workspaceId, path };
    }
    let disposed = false;
    const timeout = window.setTimeout(() => {
      if (disposed) return;
      setLoading(false);
      setError("The folder did not respond. Choose it again from the workspace rail or grant Harbor access in your system privacy settings.");
    }, 10000);
    setLoading(true);
    setError(null);
    const start = window.setTimeout(() => {
      void listWorkspace(workspaceId, path)
        .then((next) => {
          if (disposed) return;
          autoRetryRef.current = false;
          window.clearTimeout(timeout);
          setEntries(next);
          setError(null);
          setLoading(false);
        })
        .catch((reason) => {
          if (disposed) return;
          if (!autoRetryRef.current) {
            autoRetryRef.current = true;
            setLoading(true);
            window.setTimeout(() => {
              if (!disposed) setReloadKey((value) => value + 1);
            }, 250);
            return;
          }
          window.clearTimeout(timeout);
          setEntries([]);
          setError(filesystemError(reason, "The folder could not be read."));
          setLoading(false);
        });
    }, 250);
    return () => {
      disposed = true;
      window.clearTimeout(start);
      window.clearTimeout(timeout);
    };
  }, [workspaceId, path, reloadKey]);

  if (!workspaceId) return <p className="harbor-muted">Add a workspace to browse files.</p>;
  if (loading) {
    return (
      <div className="harbor-tree-status" role="status" aria-live="polite">
        <span className="harbor-spinner" aria-hidden="true" />
        <span>Reading files…</span>
      </div>
    );
  }
  if (error) {
    return (
      <div className="harbor-tree-error" role="alert">
        <strong>Folder unavailable</strong>
        <p>{error}</p>
        <button type="button" onClick={() => {
          autoRetryRef.current = false;
          setReloadKey((value) => value + 1);
        }}>Try again</button>
      </div>
    );
  }
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
