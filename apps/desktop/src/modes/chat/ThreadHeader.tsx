import { Button } from "@harbor/ui/Button";
import type { ChatMessage, ThreadRecord, Workspace } from "@harbor/schema/commands";
import { ContextMeter } from "./ContextMeter";

export function ThreadHeader({ thread, workspace, lines, attached, onNew, disabled }: {
  thread: ThreadRecord | null;
  workspace?: Workspace;
  lines: ChatMessage[];
  attached?: number;
  onNew: () => void;
  disabled?: boolean;
}) {
  return <header className="harbor-thread-header">
    <div className="harbor-thread-heading"><strong>{thread?.title ?? workspace?.title ?? "Folder conversations"}</strong><p>{workspace?.folder ?? "Other chats"}{thread ? ` · ${thread.engineId}` : ""}</p></div>
    <div className="harbor-thread-header-end">
      <ContextMeter lines={lines} attached={attached} />
      <Button disabled={disabled} onClick={onNew}>New thread</Button>
    </div>
  </header>;
}
