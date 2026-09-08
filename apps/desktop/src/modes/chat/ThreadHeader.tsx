import { Button } from "@harbor/ui/Button";
import type { ThreadRecord, Workspace } from "@harbor/schema/commands";

export function ThreadHeader({ thread, workspace, onNew, disabled }: {
  thread: ThreadRecord | null;
  workspace?: Workspace;
  onNew: () => void;
  disabled?: boolean;
}) {
  return <header className="harbor-thread-header">
    <div className="harbor-thread-heading"><strong>{thread?.title ?? workspace?.title ?? "Folder conversations"}</strong><p>{workspace?.folder ?? "Other chats"}</p></div>
    <Button disabled={disabled} onClick={onNew}>New thread</Button>
  </header>;
}
