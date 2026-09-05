import { Button } from "@harbor/ui/Button";

interface TerminalPaneProps {
  focused: boolean;
  paused: boolean;
  onFocus: () => void;
  onResume: () => void;
}

export function TerminalPane({ focused, paused, onFocus, onResume }: TerminalPaneProps) {
  return (
    <section className="harbor-pane" data-focused={focused} onClick={onFocus} aria-label="Terminal">
      <header>
        <span className="harbor-live-dot" data-on={!paused && focused} />
        Terminal
        {paused ? <Button onClick={onResume}>Resume</Button> : null}
      </header>
      <pre className="harbor-xterm">{paused ? "Restored terminal is paused." : "PTY attaches in the native host."}</pre>
    </section>
  );
}
