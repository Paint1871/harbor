import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import type { RestoredPane, Workspace, WorkspaceTab } from "@harbor/schema/commands";
import { AddWorkspace } from "../workspaces/AddWorkspace";
import { useChrome } from "./chrome-context";

function workspaceName(workspace: Workspace): string {
  if (workspace.title) return workspace.title;
  return workspace.folder.split(/[\\/]/).filter(Boolean).pop() ?? "Workspace";
}

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

function paneIcon(pane: RestoredPane, index: number): string {
  if (pane.kind === "browser") return "◎";
  if (pane.kind === "thread") return "✣";
  if (pane.kind === "files") return "⌁";
  if (pane.engineId && pane.engineId !== "shell") return "✳";
  return index === 0 ? "✳" : "›_";
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
      invoke<Workspace[]>("workspace_list").catch(() => [] as Workspace[]),
      invoke<WorkspaceTab[]>("layout_restore").catch(() => [] as WorkspaceTab[]),
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
                <RailRow
                  label={workspaceName(workspace)}
                  leading={<span aria-hidden="true">⌄</span>}
                  selected={expanded}
                  title={workspace.folder}
                  onClick={() => {
                    setExpandedWorkspaceId(workspace.id);
                    if (firstPane && onCodePaneSelect) onCodePaneSelect(workspace.id, firstPane.id);
                    else onModeChange("code");
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
                            <span className={`harbor-pane-row-icon harbor-pane-row-icon-${pane.kind}`} aria-hidden="true">{paneIcon(pane, terminalIndex < 0 ? index : terminalIndex)}</span>
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
        const tab = await invoke<WorkspaceTab>("workspace_ensure_tab", { workspaceId: workspace.id }).catch(() => null);
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
