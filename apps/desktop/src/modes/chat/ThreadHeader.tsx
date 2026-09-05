import { Button } from "@harbor/ui/Button";
import type { ThreadRecord, Workspace } from "@harbor/schema/commands";

interface ThreadHeaderProps {
  thread: ThreadRecord | null;
  workspace?: Workspace;
  onNew: () => void;
}

export function ThreadHeader({ thread, workspace, onNew }: ThreadHeaderProps) {
  return (
    <header className="harbor-thread-header">
      <div>
        <strong>{workspace?.title ?? workspace?.folder ?? "Other chats"}</strong>
        <span>{thread?.title ?? "No thread"}</span>
        <span>{thread?.engineId ?? "OpenCode"}</span>
      </div>
      <Button onClick={onNew}>New thread</Button>
    </header>
  );
}
