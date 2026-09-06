import { Composer } from "@harbor/ui/Composer";
import { PaneHeader } from "./PaneHeader";

interface ThreadPaneProps {
  state: ThreadPaneState;
  onStateChange: (update: (current: ThreadPaneState) => ThreadPaneState) => void;
  focused: boolean;
  expanded?: boolean;
  onFocus: () => void;
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
    model: "Automatic · Sonnet 5",
    effort: "Low",
    notice: "Ready",
    messages: [{ role: "user", text: "Fix the failing checkout test in auth.e2e-spec.ts" }],
  };
}

export function ThreadPane({ state, onStateChange, focused, expanded = false, onFocus, onExpand, onSplit, onClose }: ThreadPaneProps) {
  const { draft, model, effort, notice, messages } = state;

  function send(value: string) {
    const text = value.trim();
    if (!text) return;
    onStateChange((current) => ({ ...current, messages: [...current.messages, { role: "user", text }], draft: "", notice: "Queued" }));
    window.setTimeout(() => onStateChange((current) => ({ ...current, notice: "Ready" })), 700);
  }

  return (
    <section className="harbor-pane harbor-thread-pane" data-focused={focused} onClick={onFocus} aria-label="Thread">
      <PaneHeader title="Thread" live={notice === "Queued"} leading={<span className="harbor-pane-app-mark" data-kind="thread" aria-hidden="true">✣</span>} expanded={expanded} onExpand={onExpand} onSplit={onSplit} onClose={onClose} />
      <div className="harbor-thread-pane-body">
        <div className="harbor-thread-pane-history">
          {messages.map((message, index) => (
            <div className={`harbor-thread-pane-message harbor-thread-pane-message-${message.role}`} key={`${message.role}-${index}`}>
              {message.text}
            </div>
          ))}
          <div className="harbor-thread-pane-tool"><span aria-hidden="true">›</span><strong>+2 previous tool calls</strong></div>
          <div className="harbor-thread-pane-tool"><span aria-hidden="true">▣</span><span>Edit <code>src/auth/auth.service.ts</code></span><span className="harbor-thread-pane-check" aria-label="Complete">✓</span></div>
          <p className="harbor-thread-pane-summary">The refresh path re-issued a token after the session was revoked. Guard tightened.</p>
        </div>
        <Composer
          value={draft}
          onValueChange={(value) => onStateChange((current) => ({ ...current, draft: value }))}
          onSend={send}
          textareaProps={{ "aria-label": "Thread message", placeholder: "Ask anything..." }}
          controls={
            <div className="harbor-thread-pane-controls">
              <label>
                <span className="harbor-thread-pane-control-icon" aria-hidden="true">⌘</span>
                <select aria-label="Thread model" value={model} onChange={(event) => onStateChange((current) => ({ ...current, model: event.target.value }))}>
                  <option>Automatic · Sonnet 5</option>
                  <option>Automatic · Opus 4</option>
                </select>
              </label>
              <label>
                <span aria-hidden="true">ϟ</span>
                <select aria-label="Thread effort" value={effort} onChange={(event) => onStateChange((current) => ({ ...current, effort: event.target.value }))}>
                  <option>Low</option>
                  <option>Medium</option>
                  <option>High</option>
                </select>
              </label>
            </div>
          }
        />
      </div>
    </section>
  );
}
