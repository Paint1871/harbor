import { useEffect, useRef, useState } from "react";
import type { FsEntry } from "@harbor/schema/commands";
import { filesystemError, listWorkspace } from "./fs";
import { matchesFileQuery } from "./helpers";

interface TreeProps {
  workspaceId?: string;
  onOpen: (path: string) => void;
  path?: string;
  query?: string;
}

export function Tree({ workspaceId, onOpen, path = "", query }: TreeProps) {
  const [entries, setEntries] = useState<FsEntry[]>([]);
  const [openDirs, setOpenDirs] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [filter, setFilter] = useState("");
  const autoRetryRef = useRef(false);
  const scopeRef = useRef<{ workspaceId?: string; path?: string }>({});
  const activeQuery = query ?? filter;
  const showSearch = query === undefined;

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

  const visible = entries.filter((entry) => matchesFileQuery(entry.name, activeQuery));
  const listing = loading ? (
    <div className="harbor-tree-status" role="status" aria-live="polite">
      <span className="harbor-spinner" aria-hidden="true" />
      <span>Reading files…</span>
    </div>
  ) : error ? (
    <div className="harbor-tree-error" role="alert">
      <strong>Folder unavailable</strong>
      <p>{error}</p>
      <button type="button" onClick={() => {
        autoRetryRef.current = false;
        setReloadKey((value) => value + 1);
      }}>Try again</button>
    </div>
  ) : visible.length ? (
    <ul className="harbor-tree">
      {visible.map((entry) => (
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
            <Tree workspaceId={workspaceId} onOpen={onOpen} path={entry.path} query={activeQuery} />
          ) : null}
        </li>
      ))}
    </ul>
  ) : activeQuery.trim() ? (
    showSearch ? <p className="harbor-muted">No files match “{activeQuery}”.</p> : null
  ) : (
    <ul className="harbor-tree" />
  );

  if (!showSearch) return listing;
  return (
    <>
      <label className="harbor-destination-search harbor-file-tree-search">
        Search files
        <input
          aria-label="Search files"
          value={filter}
          placeholder="Find a file"
          onChange={(event) => setFilter(event.target.value)}
        />
      </label>
      {listing}
    </>
  );
}
