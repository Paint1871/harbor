import { useState, type ReactNode } from "react";
import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import type { Workspace } from "@harbor/schema/commands";

interface FolderRailProps {
  workspaces: Workspace[];
  otherCount: number;
  selectedId: string | null;
  onAddWorkspace: () => void;
  onSelect: (id: string | null) => void;
  children: ReactNode;
}

export function folderMatchesQuery(workspace: { title?: string | null; folder: string }, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  if ((workspace.title ?? "").toLowerCase().includes(needle)) return true;
  return workspace.folder.toLowerCase().includes(needle);
}

export function FolderRail({ workspaces, otherCount, selectedId, onAddWorkspace, onSelect, children }: FolderRailProps) {
  const [query, setQuery] = useState("");
  const visible = workspaces.filter((workspace) => folderMatchesQuery(workspace, query));
  const needle = query.trim();

  return <div className="harbor-chat-rail" aria-label="Workspaces">
    <div className="harbor-rail-heading"><h2>Folders</h2><Button variant="ghost" size="icon" aria-label="Add folder" onClick={onAddWorkspace}>+</Button></div>
    <label className="harbor-destination-search harbor-thread-search">Search folders<input aria-label="Search folders" value={query} placeholder="Find a folder" onChange={(event) => setQuery(event.target.value)} /></label>
    {visible.map((workspace) => <RailRow key={workspace.id} label={workspace.title ?? workspace.folder} description={workspace.folder}
      leading={<span aria-hidden="true">▱</span>} selected={selectedId === workspace.id} onClick={() => onSelect(workspace.id)} />)}
    {workspaces.length === 0 && !needle ? <p className="harbor-rail-hint">Add a project folder to keep its conversations together.</p> : null}
    {needle && visible.length === 0 ? <p className="harbor-muted">No folders match “{query}”.</p> : null}
    {otherCount > 0 ? <RailRow label="Other chats" description={`${otherCount} without a folder`} selected={selectedId === null} onClick={() => onSelect(null)} /> : null}
    <div className="harbor-thread-section"><h2>Threads</h2>{children}</div>
  </div>;
}
