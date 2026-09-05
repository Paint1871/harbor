import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Composer } from "@harbor/ui/Composer";
import { RailRow } from "@harbor/ui/RailRow";
import type { PaneLayout, Workspace } from "@harbor/schema/commands";
import { tidy } from "../../layout/tidy";
import { LaunchWizard } from "./LaunchWizard";
import { TerminalPane } from "../../panes/TerminalPane";
import { FilesPane } from "../../panes/files/FilesPane";
import { AppRail } from "../../chrome/AppRail";
import { useChrome } from "../../chrome/chrome-context";

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
}

export function CodeMode({ railOpen = true }: { railOpen?: boolean }) {
  const { registerTidy, setDestination } = useChrome();
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
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [closed, setClosed] = useState<Record<string, boolean>>({});

  const canSend = focused === "term" && !paused && !closed.term;

  useEffect(() => {
    registerTidy(() => setLayout((current) => tidy(current)));
    void invoke<Workspace[]>("workspace_list")
      .then(setWorkspaces)
      .catch(() => setWorkspaces([]));
  }, [registerTidy]);

  const workspace = workspaces[0];

  return (
    <div className="harbor-code-shell">
      {railOpen ? (
        <AppRail>
          <div className="harbor-rail-section">
            <div className="harbor-rail-heading">
              <h2>Workspace</h2>
              <Button variant="ghost" onClick={() => setWizard(true)}>
                Launch
              </Button>
            </div>
            <RailRow
              label={workspace?.title ?? workspace?.folder ?? "Workspace"}
              description={workspace?.folder ?? "No folder yet"}
              selected
              onClick={() => setDestination("mode")}
            />
            <ul className="harbor-pane-rows" aria-label="Panes">
              <li>
                <button type="button" data-selected={focused === "term"} onClick={() => setFocused("term")}>
                  Terminal
                </button>
              </li>
              <li>
                <button type="button" data-selected={focused === "files"} onClick={() => setFocused("files")}>
                  Files
                </button>
              </li>
            </ul>
          </div>
        </AppRail>
      ) : null}
      <div className="harbor-stage-panel harbor-code">
        <div
          className="harbor-code-panes"
          style={{
            gridTemplateColumns: `${layout.type === "split" ? layout.ratio * 100 : 50}% 1fr`,
          }}
        >
          {closed.term ? null : (
            <TerminalPane
              focused={focused === "term"}
              paused={paused}
              onFocus={() => setFocused("term")}
              onResume={() => setPaused(false)}
              onSplit={() => setFocused("term")}
              onClose={() => setClosed((current) => ({ ...current, term: true }))}
            />
          )}
          {closed.files ? null : (
            <FilesPane
              focused={focused === "files"}
              onFocus={() => setFocused("files")}
              onSplit={() => setFocused("files")}
              onClose={() => setClosed((current) => ({ ...current, files: true }))}
            />
          )}
        </div>
        <Composer
          value={command}
          onValueChange={setCommand}
          disabled={!canSend}
          onSend={(value) => {
            setCommand("");
            const bytes = new TextEncoder().encode(`${value}\r`);
            void invoke("pty_write_b64", { paneId: "term", b64: bytesToB64(bytes) });
          }}
          controls={
            <span className="harbor-muted">
              {canSend ? "Enter sends to the focused terminal" : "Focus a terminal"}
            </span>
          }
        />
        {wizard ? <LaunchWizard onClose={() => setWizard(false)} onLaunch={() => setWizard(false)} /> : null}
      </div>
    </div>
  );
}
