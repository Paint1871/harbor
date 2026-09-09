import { useEffect, useState } from "react";
import { call } from "../ipc";
import { Button } from "@harbor/ui/Button";
import { WorkspaceRailRow, workspaceName } from "../workspaces/WorkspaceRailRow";
import { DisclosureIcon, PaneIcon, type PaneIconKind } from "./icons";
import { EngineMark } from "./EngineMark";
import type { RestoredPane, Workspace, WorkspaceTab } from "@harbor/schema/commands";
import { AddWorkspace } from "../workspaces/AddWorkspace";
import { useChrome } from "./chrome-context";

function paneName(pane: RestoredPane, index: number): string {
  if (pane.kind === "browser") return "localhost:3000";
  if (pane.kind === "thread") return "Thread";
  if (pane.kind === "files") return "Files";
  if (pane.engineId === "shell") return index === 0 ? "Shell" : "Terminal";
  const engines: Record<string, string> = {
    "claude-code": "Claude Code",
    opencode: "OpenCode",
    codex: "Codex",
    cursor: "Cursor",
    gemini: "Gemini CLI",
    copilot: "GitHub Copilot",
    "grok-build": "Grok Build",
    "kimi-code": "Kimi Code",
    "muse-code": "Muse Code",
  };
  const engineLabel = pane.engineId ? engines[pane.engineId] : undefined;
  if (engineLabel) return engineLabel;
  return index === 0 ? "Claude Code" : index === 1 ? "zsh 104×31" : "Terminal";
}

function paneIconKind(pane: RestoredPane, index: number): PaneIconKind {
  if (pane.kind === "browser") return "browser";
  if (pane.kind === "thread") return "thread";
  if (pane.kind === "files") return "files";
  if (pane.engineId && pane.engineId !== "shell") return "agent";
  return index === 0 ? "agent" : "terminal";
}

function orderedPanes(panes: RestoredPane[]): RestoredPane[] {
  const terminals = panes.filter((pane) => pane.kind === "terminal");
  const rank = (pane: RestoredPane) => {
    if (pane.kind === "terminal") return terminals.indexOf(pane);
    if (pane.kind === "browser") return 2;
    if (pane.kind === "thread") return 3;
    return 4;
  };
  return [...panes].sort((a, b) => rank(a) - rank(b));
}

export function DestinationWorkspaceRail() {
  const { onCodePaneSelect, onModeChange } = useChrome();
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [tabs, setTabs] = useState<WorkspaceTab[]>([]);
  const [addingWorkspace, setAddingWorkspace] = useState(false);
  const [expandedWorkspaceId, setExpandedWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    void Promise.all([
      call("workspace_list").catch(() => [] as Workspace[]),
      call("layout_restore").catch(() => [] as WorkspaceTab[]),
    ]).then(([listed, restored]) => {
      if (disposed) return;
      const nextWorkspaces = Array.isArray(listed) ? listed : [];
      setWorkspaces(nextWorkspaces);
      setTabs(Array.isArray(restored) ? restored : []);
      setExpandedWorkspaceId((current) => current && nextWorkspaces.some((workspace) => workspace.id === current) ? current : nextWorkspaces[0]?.id ?? null);
    });
    return () => {
      disposed = true;
    };
  }, []);

  return (
    <>
      <div className="harbor-destination-workspace-rail">
        <div className="harbor-rail-section">
          <div className="harbor-rail-heading">
            <h2>Workspaces</h2>
            <Button size="icon" variant="ghost" aria-label="Add to Workspaces" title="Add to Workspaces" onClick={() => setAddingWorkspace(true)}>
              <span aria-hidden="true">+</span>
            </Button>
          </div>
          {workspaces.length ? workspaces.map((workspace) => {
            const tab = tabs.find((item) => item.workspaceId === workspace.id);
            const terminals = tab?.panes.filter((pane) => pane.kind === "terminal") ?? [];
            const expanded = expandedWorkspaceId === workspace.id;
            const firstPane = tab ? orderedPanes(tab.panes)[0] : undefined;
            return (
              <div className="harbor-destination-workspace" key={workspace.id}>
                <WorkspaceRailRow
                  workspace={workspace}
                  leading={<DisclosureIcon open={expanded} />}
                  selected={expanded}
                  onSelect={() => {
                    setExpandedWorkspaceId(workspace.id);
                    if (firstPane && onCodePaneSelect) onCodePaneSelect(workspace.id, firstPane.id);
                    else onModeChange("code");
                  }}
                  onRenamed={(updated) =>
                    setWorkspaces((rows) => rows.map((row) => (row.id === updated.id ? updated : row)))
                  }
                  onRemoved={(id) => {
                    const rest = workspaces.filter((row) => row.id !== id);
                    setWorkspaces(rest);
                    setTabs((rows) => rows.filter((row) => row.workspaceId !== id));
                    setExpandedWorkspaceId((current) => (current === id ? rest[0]?.id ?? null : current));
                  }}
                />
                {expanded && tab?.panes.length ? (
                  <ul className="harbor-destination-pane-rows" aria-label={`${workspaceName(workspace)} panes`}>
                    {orderedPanes(tab.panes).map((pane, index) => {
                      const terminalIndex = terminals.indexOf(pane);
                      return (
                        <li key={pane.id}>
                          <button type="button" onClick={() => {
                            setExpandedWorkspaceId(workspace.id);
                            if (onCodePaneSelect) onCodePaneSelect(workspace.id, pane.id);
                            else onModeChange("code");
                          }}>
                            <span className="harbor-pane-row-icon">{pane.kind === "terminal" ? <EngineMark engineId={pane.engineId} label={paneName(pane, terminalIndex < 0 ? index : terminalIndex)} size={13} /> : <PaneIcon kind={paneIconKind(pane, terminalIndex < 0 ? index : terminalIndex)} />}</span>
                            {paneName(pane, terminalIndex < 0 ? index : terminalIndex)}
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                ) : null}
              </div>
            );
          }) : <p className="harbor-rail-empty">Add a folder to start coding.</p>}
        </div>
      </div>
      {addingWorkspace ? <AddWorkspace onClose={() => setAddingWorkspace(false)} onAdded={async (workspace) => {
        setAddingWorkspace(false);
        setWorkspaces((current) => current.some((item) => item.id === workspace.id) ? current : [...current, workspace]);
        setExpandedWorkspaceId(workspace.id);
        const tab = await call("workspace_ensure_tab", { workspaceId: workspace.id }).catch(() => null);
        if (tab) {
          setTabs((current) => [...current.filter((item) => item.workspaceId !== workspace.id), tab]);
          const pane = orderedPanes(tab.panes)[0];
          if (pane && onCodePaneSelect) {
            onCodePaneSelect(workspace.id, pane.id);
            return;
          }
        }
        onModeChange("code");
      }} /> : null}
    </>
  );
}
