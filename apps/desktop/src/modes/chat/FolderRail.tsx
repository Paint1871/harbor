import type { ReactNode } from "react";
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

export function FolderRail({ workspaces, otherCount, selectedId, onAddWorkspace, onSelect, children }: FolderRailProps) {
  return <div className="harbor-chat-rail" aria-label="Workspaces">
    <div className="harbor-rail-heading"><h2>Folders</h2><Button variant="ghost" size="icon" aria-label="Add folder" onClick={onAddWorkspace}>+</Button></div>
    {workspaces.map((workspace) => <RailRow key={workspace.id} label={workspace.title ?? workspace.folder} description={workspace.folder}
      leading={<span aria-hidden="true">▱</span>} selected={selectedId === workspace.id} onClick={() => onSelect(workspace.id)} />)}
    {workspaces.length === 0 ? <p className="harbor-rail-hint">Add a project folder to keep its conversations together.</p> : null}
    {otherCount > 0 ? <RailRow label="Other chats" description={`${otherCount} without a folder`} selected={selectedId === null} onClick={() => onSelect(null)} /> : null}
    <div className="harbor-thread-section"><h2>Threads</h2>{children}</div>
  </div>;
}
