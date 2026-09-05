import { useState } from "react";
import { Button } from "@harbor/ui/Button";
import { Composer } from "@harbor/ui/Composer";
import type { PaneLayout } from "@harbor/schema/commands";
import { tidy } from "../../layout/tidy";
import { LaunchWizard } from "./LaunchWizard";
import { TerminalPane } from "../../panes/TerminalPane";
import { FilesPane } from "../../panes/files/FilesPane";

export function CodeMode() {
  const [layout, setLayout] = useState<PaneLayout>({
    type: "split",
    dir: "h",
    ratio: 0.5,
    a: { type: "leaf", paneId: "term" },
    b: { type: "leaf", paneId: "files" },
  });
  const [focused, setFocused] = useState<"term" | "files" | null>("term");
  const [command, setCommand] = useState("");
  const [wizard, setWizard] = useState(false);
  const [paused, setPaused] = useState(false);

  const canSend = focused === "term" && !paused;

  return (
    <div className="harbor-code">
      <div className="harbor-code-toolbar">
        <Button onClick={() => setWizard(true)}>Launch</Button>
        <Button onClick={() => setLayout(tidy(layout))}>Tidy Panes</Button>
        <span className="harbor-muted">{canSend ? "Command bar → focused PTY" : "Focus a terminal"}</span>
      </div>
      <div className="harbor-code-panes" style={{ gridTemplateColumns: `${layout.type === "split" ? layout.ratio * 100 : 50}% 1fr` }}>
        <TerminalPane focused={focused === "term"} paused={paused} onFocus={() => setFocused("term")} onResume={() => setPaused(false)} />
        <FilesPane focused={focused === "files"} onFocus={() => setFocused("files")} />
      </div>
      <Composer
        value={command}
        onValueChange={setCommand}
        disabled={!canSend}
        onSend={(value) => {
          setCommand("");
          void value;
        }}
        controls={<span className="harbor-muted">{canSend ? "Enter sends to the focused terminal" : "Focus a terminal"}</span>}
      />
      {wizard ? <LaunchWizard onClose={() => setWizard(false)} onLaunch={() => setWizard(false)} /> : null}
    </div>
  );
}
