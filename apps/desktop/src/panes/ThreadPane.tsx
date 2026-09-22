import { Button } from "@harbor/ui/Button";
import { PaneHeader } from "./PaneHeader";
import { useChrome } from "../chrome/chrome-context";

interface ThreadPaneProps {
  state: ThreadPaneState;
  onStateChange: (update: (current: ThreadPaneState) => ThreadPaneState) => void;
  focused: boolean;
  expanded?: boolean;
  onFocus?: () => void;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

export interface ThreadMessage {
  role: "user" | "system";
  text: string;
}

export interface ThreadPaneState {
  draft: string;
  model: string;
  effort: string;
  notice: string;
  messages: ThreadMessage[];
}

export function createThreadPaneState(): ThreadPaneState {
  return {
    draft: "",
    model: "",
    effort: "",
    notice: "",
    messages: [],
  };
}

/**
 * Folder threads live in Chat mode (ACP). This pane is a doorway to them, not
 * a second chat surface: there is no thread backend behind a Code pane, so the
 * composer sends nothing.
 */
export function ThreadPane({ focused, expanded = false, onFocus, onExpand, onSplit, onClose }: ThreadPaneProps) {
  const { onModeChange } = useChrome();
  return (
    <section className="harbor-pane harbor-thread-pane" data-focused={focused} onClick={onFocus} aria-label="Thread">
      <PaneHeader title="Thread" leading={<span className="harbor-pane-app-mark" data-kind="thread" aria-hidden="true">✣</span>} expanded={expanded} onExpand={onExpand} onSplit={onSplit} onClose={onClose} />
      <div className="harbor-thread-pane-body">
        <div className="harbor-terminal-state" role="status">
          <span className="harbor-terminal-state-mark" aria-hidden="true">✣</span>
          <strong>Threads live in Chat mode</strong>
          <p>Pick a folder there and start an ACP conversation against the same workspace.</p>
          <Button variant="primary" onClick={() => onModeChange("chat")}>Open Chat</Button>
        </div>
      </div>
    </section>
  );
}
