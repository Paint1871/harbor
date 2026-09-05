import type { ReactNode } from "react";
import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import type { Workspace } from "@harbor/schema/commands";

interface FolderRailProps {
  workspaces: Workspace[];
  otherCount: number;
  selectedId: string | null;
  folderInput: string;
  onFolderInput: (value: string) => void;
  onAddWorkspace: () => void;
  onSelect: (id: string | null) => void;
  children: ReactNode;
}

export function FolderRail({
  workspaces,
  otherCount,
  selectedId,
  folderInput,
  onFolderInput,
  onAddWorkspace,
  onSelect,
  children,
}: FolderRailProps) {
  return (
    <aside className="harbor-chat-rail" aria-label="Workspaces">
      <h2>Workspaces</h2>
      {workspaces.map((workspace) => (
        <RailRow
          key={workspace.id}
          label={workspace.title ?? workspace.folder}
          description={workspace.folder}
          selected={selectedId === workspace.id}
          onClick={() => onSelect(workspace.id)}
        />
      ))}
      <div className="harbor-chat-add">
        <input
          aria-label="Add workspace folder"
          placeholder="Add folder"
          value={folderInput}
          onChange={(event) => onFolderInput(event.target.value)}
        />
        <Button onClick={onAddWorkspace}>Add folder</Button>
      </div>
      {children}
      <RailRow
        label="Other chats"
        description={`${otherCount} without a folder`}
        selected={selectedId === null}
        onClick={() => onSelect(null)}
      />
    </aside>
  );
}
