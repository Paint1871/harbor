import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine, PaneLayout, PaneState, RestoredPane, Workspace, WorkspaceTab } from "@harbor/schema/commands";
import { dropLeaf, leafPaneIds, preferredSplitDir, restoredLayoutPaused, splitLeaf } from "../../layout/restore";
import { tidy } from "../../layout/tidy";
import { TerminalPane } from "../../panes/TerminalPane";
import { FilesPane } from "../../panes/files/FilesPane";
import { BrowserPane, createBrowserPaneState, type BrowserPaneState } from "../../panes/BrowserPane";
import { createThreadPaneState, ThreadPane, type ThreadPaneState } from "../../panes/ThreadPane";
import { AppRail } from "../../chrome/AppRail";
import { DisclosureIcon, PaneIcon, type PaneIconKind } from "../../chrome/icons";
import { EngineMark } from "../../chrome/EngineMark";
import { useChrome } from "../../chrome/chrome-context";
import { settingsSet } from "../../settings";
import { AddWorkspace } from "../../workspaces/AddWorkspace";
import { WorkspaceRailRow } from "../../workspaces/WorkspaceRailRow";

const DEFAULT_LAYOUT: PaneLayout = {
  type: "split",
  dir: "h",
  ratio: 0.5,
  a: { type: "leaf", paneId: "term" },
  b: { type: "leaf", paneId: "files" },
};

type CodePaneKind = "terminal" | "files" | "browser" | "thread";

function paneKind(id: string, panes: RestoredPane[]): CodePaneKind {
  const kind = panes.find((pane) => pane.id === id)?.kind;
  if (kind === "files" || kind === "browser" || kind === "thread") return kind;
  return "terminal";
}

function referenceLayout(terminalId: string, secondaryTerminalId: string, browserId: string, threadId: string): PaneLayout {
  return {
    type: "split",
    dir: "h",
    ratio: 0.53,
    a: { type: "leaf", paneId: terminalId },
    b: {
      type: "split",
      dir: "v",
      ratio: 0.27,
      a: { type: "leaf", paneId: secondaryTerminalId },
      b: {
        type: "split",
        dir: "v",
        ratio: 0.5,
        a: { type: "leaf", paneId: browserId },
        b: { type: "leaf", paneId: threadId },
      },
    },
  };
}

function hasOldReferenceRatios(layout: PaneLayout): boolean {
  if (layout.type !== "split" || layout.dir !== "h" || layout.ratio < 0.56) return false;
  if (layout.b.type !== "split" || layout.b.dir !== "v" || layout.b.ratio < 0.28) return false;
  return layout.b.b.type === "split" && layout.b.b.dir === "v" && layout.b.b.ratio > 0.52;
}

function tidyCodeLayout(layout: PaneLayout, panes: RestoredPane[]): PaneLayout {
  const leaves = leafPaneIds(layout);
  const terminals = panes.filter((pane) => pane.kind === "terminal" && leaves.includes(pane.id));
  const browser = panes.find((pane) => pane.kind === "browser" && leaves.includes(pane.id));
  const thread = panes.find((pane) => pane.kind === "thread" && leaves.includes(pane.id));
  const primary = terminals[0];
  const secondary = terminals[1];
  if (primary && secondary && browser && thread) {
    return referenceLayout(primary.id, secondary.id, browser.id, thread.id);
  }
  return tidy(layout);
}

function codeError(reason: unknown, fallback: string): string {
  const text = String(reason).replace(/^Error:\s*/i, "").trim();
  return text || fallback;
}

function SplitPanes({
  layout,
  onChange,
  renderSide,
}: {
  layout: Extract<PaneLayout, { type: "split" }>;
  onChange: (next: Extract<PaneLayout, { type: "split" }>) => void;
  renderSide: (node: PaneLayout, onChange: (next: PaneLayout) => void) => ReactNode;
}) {
  const start = useRef({ pos: 0, ratio: layout.ratio, size: 1 });
  const horizontal = layout.dir !== "v";
  return (
    <div
      className="harbor-code-panes"
      style={
        horizontal
          ? { gridTemplateColumns: `minmax(120px, ${layout.ratio * 100}%) 6px minmax(120px, 1fr)` }
          : { gridTemplateRows: `minmax(120px, ${layout.ratio * 100}%) 6px minmax(120px, 1fr)`, gridTemplateColumns: "1fr" }
      }
    >
      {renderSide(layout.a, (a) => onChange({ ...layout, a }))}
      <button
        type="button"
        className="harbor-pane-divider"
        data-dir={horizontal ? "h" : "v"}
        aria-label="Resize panes"
        onPointerDown={(event: ReactPointerEvent<HTMLButtonElement>) => {
          const parent = event.currentTarget.parentElement;
          start.current = {
            pos: horizontal ? event.clientX : event.clientY,
            ratio: layout.ratio,
            size: Math.max(1, horizontal ? parent?.clientWidth ?? 1 : parent?.clientHeight ?? 1),
          };
          const move = (ev: PointerEvent) => {
            const delta = ((horizontal ? ev.clientX : ev.clientY) - start.current.pos) / start.current.size;
            const ratio = Math.min(0.85, Math.max(0.15, start.current.ratio + delta));
            onChange({ ...layout, ratio });
          };
          const up = () => {
            window.removeEventListener("pointermove", move);
            window.removeEventListener("pointerup", up);
          };
          window.addEventListener("pointermove", move);
          window.addEventListener("pointerup", up);
        }}
      />
      {renderSide(layout.b, (b) => onChange({ ...layout, b }))}
    </div>
  );
}

export function CodeMode({
  railOpen = true,
  onPaneSelectRegister,
}: {
  railOpen?: boolean;
  onPaneSelectRegister?: (handler: (workspaceId: string, paneId: string) => void) => void;
}) {
  const { registerTidy, setDestination } = useChrome();
  const [layout, setLayout] = useState<PaneLayout>(DEFAULT_LAYOUT);
  const [focused, setFocused] = useState<string | null>("term");
  const [layoutError, setLayoutError] = useState<string | null>(null);
  const [addingWorkspace, setAddingWorkspace] = useState(false);
  const [paused, setPaused] = useState<Record<string, boolean>>({});
  const [browserStates, setBrowserStates] = useState<Record<string, BrowserPaneState>>({});
  const [threadStates, setThreadStates] = useState<Record<string, ThreadPaneState>>({});
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [panes, setPanes] = useState<RestoredPane[]>([]);
  const [tabId, setTabId] = useState<string | null>(null);
  const [expandedPaneId, setExpandedPaneId] = useState<string | null>(null);
  const [paneAddOpen, setPaneAddOpen] = useState(false);
  const [collapsedWorkspaces, setCollapsedWorkspaces] = useState<Record<string, boolean>>({});
  const [detectedEngines, setDetectedEngines] = useState<DetectedEngine[]>([]);
  const tabIdRef = useRef<string | null>(null);
  const workspacesRef = useRef(workspaces);
  const panesRef = useRef(panes);
  const activeWorkspaceIdRef = useRef<string | null>(null);
  const workspaceRequestRef = useRef(0);
  const layoutRef = useRef(layout);
  const panesRootRef = useRef<HTMLDivElement>(null);
  const layoutActionRef = useRef(0);
  const layoutWritesRef = useRef<Promise<void>>(Promise.resolve());
  const engineChangeRef = useRef<Record<string, number>>({});
  const userSelectedWorkspaceRef = useRef(false);
  const paneAddRef = useRef<HTMLLIElement>(null);
  tabIdRef.current = tabId;
  workspacesRef.current = workspaces;
  panesRef.current = panes;
  activeWorkspaceIdRef.current = activeWorkspaceId;
  layoutRef.current = layout;

  const workspace = workspaces.find((item) => item.id === activeWorkspaceId) ?? workspaces[0];
  const leaves = leafPaneIds(layout);

  const refreshTerminalEngines = useCallback(async () => {
    if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
    try {
      const next = await invoke<DetectedEngine[]>("engines_detect");
      setDetectedEngines(Array.isArray(next) ? next : []);
    } catch {
      setDetectedEngines([]);
    }
  }, []);

  useEffect(() => {
    void refreshTerminalEngines();
  }, [refreshTerminalEngines]);

  useEffect(() => {
    if (!paneAddOpen) return;
    const close = (event: PointerEvent) => {
      if (!paneAddRef.current?.contains(event.target as Node)) setPaneAddOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [paneAddOpen]);

  const persistLayout = useCallback((id: string, next: PaneLayout) => {
    const write = layoutWritesRef.current
      .catch(() => undefined)
      .then(() => invoke("workspace_save_layout", { tabId: id, layout: next }))
      .then(() => undefined);
    layoutWritesRef.current = write;
    return write;
  }, []);

  const applyTab = useCallback((tab: WorkspaceTab) => {
    ++layoutActionRef.current;
    const nextLayout = restoredLayoutPaused(tab.layout);
    setTabId(tab.id);
    setLayout(nextLayout);
    setPanes(tab.panes);
    setExpandedPaneId(null);
    setPaused(Object.fromEntries(tab.panes.filter((pane) => pane.kind === "terminal").map((pane) => [pane.id, pane.paused])));
    setBrowserStates((current) => Object.fromEntries(
      tab.panes
        .filter((pane) => pane.kind === "browser")
        .map((pane) => [pane.id, current[pane.id] ?? createBrowserPaneState()]),
    ));
    setThreadStates((current) => Object.fromEntries(
      tab.panes
        .filter((pane) => pane.kind === "thread")
        .map((pane) => [pane.id, current[pane.id] ?? createThreadPaneState()]),
    ));
    const leaves = leafPaneIds(nextLayout);
    setFocused((current) => (current && leaves.includes(current) ? current : leaves[0] ?? null));
  }, []);

  const prepareTab = useCallback(async (tab: WorkspaceTab, workspace: Workspace): Promise<WorkspaceTab> => {
    const native = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    const terminals = tab.panes.filter((pane) => pane.kind === "terminal");
    const isLegacy = native && tab.panes.length <= 2 && terminals.length === 1 && tab.panes.some((pane) => pane.kind === "files");
    if (!isLegacy) {
      const secondaryTerminal = terminals[1];
      const browser = tab.panes.find((pane) => pane.kind === "browser");
      const thread = tab.panes.find((pane) => pane.kind === "thread");
      if (native && terminals[0] && secondaryTerminal && browser && thread && hasOldReferenceRatios(tab.layout)) {
        const nextLayout = referenceLayout(terminals[0].id, secondaryTerminal.id, browser.id, thread.id);
        await invoke("workspace_save_layout", { tabId: tab.id, layout: nextLayout });
        return { ...tab, layout: nextLayout };
      }
      return tab;
    }
    const primaryTerminal = terminals[0];
    if (!primaryTerminal) return tab;

    const created: string[] = [];
    try {
      const create = async (kind: CodePaneKind) => {
        const id = await invoke<string>("pane_create", {
          tabId: tab.id,
          kind,
          state: {
            kind,
            cwd: workspace.folder,
            paused: false,
            ...(kind === "terminal" && primaryTerminal.engineId ? { engineId: primaryTerminal.engineId } : {}),
          } satisfies PaneState,
        });
        created.push(id);
        return id;
      };
      const secondaryTerminalId = await create("terminal");
      const browserId = await create("browser");
      const threadId = await create("thread");
      const legacyFiles = tab.panes.find((pane) => pane.kind === "files");
      if (legacyFiles) await invoke("pane_close", { id: legacyFiles.id });
      const nextLayout = referenceLayout(primaryTerminal.id, secondaryTerminalId, browserId, threadId);
      await invoke("workspace_save_layout", { tabId: tab.id, layout: nextLayout });
      return {
        ...tab,
        layout: nextLayout,
        panes: tab.panes.filter((pane) => pane.id !== legacyFiles?.id).concat(
          { id: secondaryTerminalId, kind: "terminal", paused: false, engineId: primaryTerminal.engineId },
          { id: browserId, kind: "browser", paused: false },
          { id: threadId, kind: "thread", paused: false },
        ),
      };
    } catch (reason) {
      await Promise.all(created.map((id) => invoke("pane_close", { id }).catch(() => undefined)));
      setLayoutError(codeError(reason, "The reference code layout could not be prepared."));
      return tab;
    }
  }, []);

  const openWorkspace = useCallback(async (workspace: Workspace) => {
    userSelectedWorkspaceRef.current = true;
    const request = workspaceRequestRef.current + 1;
    workspaceRequestRef.current = request;
    setActiveWorkspaceId(workspace.id);
    try {
      const tab = await invoke<WorkspaceTab>("workspace_ensure_tab", { workspaceId: workspace.id });
      if (workspaceRequestRef.current !== request) return null;
      applyTab(await prepareTab(tab, workspace));
      await settingsSet(`code_tab:${workspace.id}`, tab.id);
      return tab.id;
    } catch {
      return null;
    }
  }, [applyTab, prepareTab]);

  const ensureTab = useCallback(async (listed?: Workspace[]) => {
    const rows = listed ?? workspacesRef.current;
    const workspace = rows.find((item) => item.id === activeWorkspaceIdRef.current) ?? rows[0];
    if (!workspace) return null;
    return openWorkspace(workspace);
  }, [openWorkspace]);

  const focusPane = useCallback((workspaceId: string, paneId: string) => {
    const row = workspacesRef.current.find((item) => item.id === workspaceId);
    if (!row) return;
    if (activeWorkspaceIdRef.current === workspaceId) {
      if (leafPaneIds(layoutRef.current).includes(paneId)) setFocused(paneId);
      return;
    }
    void openWorkspace(row).then(() => {
      if (activeWorkspaceIdRef.current === workspaceId) setFocused(paneId);
    });
  }, [openWorkspace]);

  useEffect(() => {
    if (!onPaneSelectRegister) return;
    onPaneSelectRegister(focusPane);
    return () => onPaneSelectRegister(() => undefined);
  }, [focusPane, onPaneSelectRegister]);

  useEffect(() => {
    registerTidy(() => {
      ++layoutActionRef.current;
      setLayoutError(null);
      setLayout((current) => tidyCodeLayout(current, panesRef.current));
    });
  }, [registerTidy]);

  useEffect(() => {
    void (async () => {
      try {
        const restored = await invoke<WorkspaceTab[]>("layout_restore");
        const listed = await invoke<Workspace[]>("workspace_list").catch(() => [] as Workspace[]);
        setWorkspaces(listed);
        const workspace = listed[0];
        if (workspace && !userSelectedWorkspaceRef.current) setActiveWorkspaceId(workspace.id);
        const tab = workspace ? restored.find((item) => item.workspaceId === workspace.id) : restored[0];
        if (tab && !userSelectedWorkspaceRef.current) {
          applyTab(workspace ? await prepareTab(tab, workspace) : tab);
          if (workspace) await settingsSet(`code_tab:${workspace.id}`, tab.id);
        } else if (!userSelectedWorkspaceRef.current) {
          await ensureTab(listed);
        }
      } catch {
        const listed = await invoke<Workspace[]>("workspace_list").catch(() => [] as Workspace[]);
        setWorkspaces(listed);
        await ensureTab(listed);
      }
    })();
  }, [applyTab, ensureTab, prepareTab]);

  useEffect(() => {
    if (!tabId) return;
    void persistLayout(tabId, layout).catch((reason) => {
      setLayoutError(codeError(reason, "The pane layout could not be saved."));
    });
  }, [layout, persistLayout, tabId]);

  async function closePane(paneId: string) {
    const next = dropLeaf(layoutRef.current, paneId);
    if (!next) return;
    ++layoutActionRef.current;
    setLayoutError(null);
    setLayout(next);
    setPanes((current) => current.filter((pane) => pane.id !== paneId));
    setExpandedPaneId((current) => (current === paneId ? null : current));
    setPaused((current) => {
      const copy = { ...current };
      delete copy[paneId];
      return copy;
    });
    setBrowserStates((current) => {
      const copy = { ...current };
      delete copy[paneId];
      return copy;
    });
    setThreadStates((current) => {
      const copy = { ...current };
      delete copy[paneId];
      return copy;
    });
    if (tabId) {
      void invoke("pane_close", { id: paneId }).catch((reason) => {
        setLayoutError(codeError(reason, "The pane could not be closed."));
      });
    }
    if (focused === paneId) setFocused(leafPaneIds(next)[0] ?? null);
  }

  /** Split along the pane's long axis so the two halves stay usable. */
  function splitDirectionFor(paneId: string): "h" | "v" {
    const grid = panesRootRef.current?.getBoundingClientRect();
    return preferredSplitDir(layoutRef.current, paneId, grid?.width ?? 1, grid?.height ?? 1);
  }

  async function createPane(
    kind: CodePaneKind,
    paneId = focused ?? leaves[0],
    engineId?: string,
  ) {
    if (!tabId || !workspace || !paneId) return;
    const action = ++layoutActionRef.current;
    setLayoutError(null);
    const state: PaneState = { kind, cwd: workspace.folder, paused: false };
    if (kind === "terminal") {
      state.engineId = engineId
        ?? panes.find((pane) => pane.id === paneId && pane.kind === "terminal")?.engineId
        ?? panes.find((pane) => pane.kind === "terminal")?.engineId
        ?? "shell";
    }
    const dir = splitDirectionFor(paneId);
    try {
      const created = await invoke<string>("pane_create", { tabId, kind, state });
      const next = splitLeaf(layoutRef.current, paneId, created, dir);
      if (layoutActionRef.current !== action || !leafPaneIds(next).includes(created)) {
        await invoke("pane_close", { id: created }).catch(() => undefined);
        return;
      }
      setPanes((current) => [...current, { id: created, kind, paused: false, engineId: state.engineId }]);
      setLayout(next);
      setFocused(created);
      if (kind === "browser") {
        setBrowserStates((current) => ({ ...current, [created]: createBrowserPaneState() }));
      }
      if (kind === "thread") {
        setThreadStates((current) => ({ ...current, [created]: createThreadPaneState() }));
      }
      if (kind === "terminal") {
        setPaused((current) => ({ ...current, [created]: false }));
      }
    } catch (reason) {
      setLayoutError(codeError(reason, `The ${kind} pane could not be opened.`));
    }
  }

  const changeTerminalEngine = useCallback(async (paneId: string, engineId: string) => {
    const pane = panesRef.current.find((item) => item.id === paneId);
    if (!pane || pane.kind !== "terminal") return;
    const action = (engineChangeRef.current[paneId] ?? 0) + 1;
    engineChangeRef.current[paneId] = action;
    setLayoutError(null);
    try {
      await invoke("pane_set_engine", { id: paneId, engineId });
      if (engineChangeRef.current[paneId] !== action) return;
      setPanes((current) => current.map((item) => item.id === paneId ? { ...item, engineId } : item));
    } catch (reason) {
      if (engineChangeRef.current[paneId] === action) setLayoutError(codeError(reason, "The terminal CLI could not be changed."));
      throw reason;
    }
  }, []);

  function terminalPaneLabel(paneId: string): string {
    const terminalIndex = leaves.filter((id) => paneKind(id, panes) === "terminal").indexOf(paneId);
    const pane = panes.find((item) => item.id === paneId);
    if (pane?.engineId && pane.engineId !== "shell") {
      return {
        "claude-code": "Claude Code",
        opencode: "OpenCode",
        codex: "Codex",
        gemini: "Gemini CLI",
        copilot: "GitHub Copilot",
      }[pane.engineId] ?? pane.engineId;
    }
    if (pane?.engineId === "shell") return terminalIndex === 0 ? "Shell" : "Terminal";
    return terminalIndex === 0 ? "Claude Code" : terminalIndex === 1 ? "zsh 104×31" : "Terminal";
  }

  function renderPane(paneId: string): ReactNode {
    const kind = paneKind(paneId, panes);
    if (kind === "files") {
      return (
        <FilesPane
          key={`${paneId}:${workspace?.id ?? "no-workspace"}`}
          workspaceId={workspace?.id}
          focused={focused === paneId}
          expanded={expandedPaneId === paneId}
          onFocus={() => setFocused(paneId)}
          onExpand={() => setExpandedPaneId((current) => (current === paneId ? null : paneId))}
          onSplit={() => void createPane("files", paneId)}
          onClose={() => void closePane(paneId)}
        />
      );
    }
    if (kind === "browser") {
      return (
        <BrowserPane
          key={`${paneId}:${workspace?.id ?? "no-workspace"}`}
          state={browserStates[paneId] ?? createBrowserPaneState()}
          onStateChange={(update) => setBrowserStates((current) => ({
            ...current,
            [paneId]: update(current[paneId] ?? createBrowserPaneState()),
          }))}
          focused={focused === paneId}
          expanded={expandedPaneId === paneId}
          onFocus={() => setFocused(paneId)}
          onExpand={() => setExpandedPaneId((current) => (current === paneId ? null : paneId))}
          onSplit={() => void createPane("browser", paneId)}
          onClose={() => void closePane(paneId)}
        />
      );
    }
    if (kind === "thread") {
      return (
        <ThreadPane
          key={`${paneId}:${workspace?.id ?? "no-workspace"}`}
          state={threadStates[paneId] ?? createThreadPaneState()}
          onStateChange={(update) => setThreadStates((current) => ({
            ...current,
            [paneId]: update(current[paneId] ?? createThreadPaneState()),
          }))}
          focused={focused === paneId}
          expanded={expandedPaneId === paneId}
          onFocus={() => setFocused(paneId)}
          onExpand={() => setExpandedPaneId((current) => (current === paneId ? null : paneId))}
          onSplit={() => void createPane("thread", paneId)}
          onClose={() => void closePane(paneId)}
        />
      );
    }
    return (
      <TerminalPane
        key={`${paneId}:${workspace?.id ?? "no-workspace"}`}
        paneId={paneId}
        label={terminalPaneLabel(paneId)}
        engineId={panes.find((pane) => pane.id === paneId)?.engineId}
        availableEngines={detectedEngines}
        onEngineChange={(engineId) => changeTerminalEngine(paneId, engineId)}
        workspaceId={workspace?.id}
        focused={focused === paneId}
        expanded={expandedPaneId === paneId}
        paused={!!paused[paneId]}
        onFocus={() => setFocused(paneId)}
        onExpand={() => setExpandedPaneId((current) => (current === paneId ? null : paneId))}
        onResume={() => {
          setPaused((current) => ({ ...current, [paneId]: false }));
        }}
        onSplit={() => void createPane("terminal", paneId)}
        onClose={() => void closePane(paneId)}
      />
    );
  }

  function renderLayout(node: PaneLayout, onChange: (next: PaneLayout) => void): ReactNode {
    if (expandedPaneId) return renderPane(expandedPaneId);
    if (node.type === "tabs") {
      const active = node.kids[node.active] ?? node.kids[0];
      return active ? renderLayout({ type: "leaf", paneId: active }, onChange) : null;
    }
    if (node.type === "split") {
      return (
        <SplitPanes
          layout={node}
          onChange={onChange}
          renderSide={renderLayout}
        />
      );
    }
    return renderPane(node.paneId);
  }

  /** Every installed terminal CLI is one click away, plus a plain shell. */
  const terminalStarters = [
    ...detectedEngines
      .filter((engine) => engine.supportsTerminal !== false && engine.status !== "cli-missing" && Boolean(engine.path))
      .map((engine) => ({ engineId: engine.id, label: engine.displayName })),
    { engineId: "shell", label: "Shell" },
  ];
  const paneLabel = (kind: CodePaneKind) => ({ terminal: "Terminal", files: "Files", browser: "localhost:3000", thread: "Thread" })[kind];
  const paneRowLabel = (paneId: string) => {
    const kind = paneKind(paneId, panes);
    return kind === "terminal" ? terminalPaneLabel(paneId) : paneLabel(kind);
  };
  const paneRowIconKind = (paneId: string): PaneIconKind => {
    const kind = paneKind(paneId, panes);
    if (kind !== "terminal") return kind;
    // A terminal running an engine reads as an agent seat, a bare shell does not.
    return terminalPaneLabel(paneId) === "Claude Code" ? "agent" : "terminal";
  };

  return (
    <div className="harbor-code-shell">
      <AppRail open={railOpen}>
          <div className="harbor-rail-section">
            <div className="harbor-rail-heading">
              <h2>Workspaces</h2>
              <Button size="icon" variant="ghost" aria-label="Add to Workspaces" title="Add to Workspaces" onClick={() => setAddingWorkspace(true)}>
                <span aria-hidden="true">+</span>
              </Button>
            </div>
            {workspaces.length ? workspaces.map((row) => {
              const active = row.id === workspace?.id;
              const open = active && !collapsedWorkspaces[row.id];
              return (
                <div className="harbor-code-workspace" key={row.id}>
                  <WorkspaceRailRow
                    className="harbor-code-workspace-row"
                    workspace={row}
                    description={row.folder}
                    leading={<DisclosureIcon open={open} />}
                    selected={active}
                    onSelect={() => {
                      setDestination("mode");
                      // Opening a folder is also what reveals its panes.
                      if (active) {
                        setCollapsedWorkspaces((current) => ({ ...current, [row.id]: !current[row.id] }));
                      } else {
                        setCollapsedWorkspaces((current) => ({ ...current, [row.id]: false }));
                        void openWorkspace(row);
                      }
                    }}
                    onRenamed={(updated) => {
                      setWorkspaces((rows) => rows.map((item) => (item.id === updated.id ? updated : item)));
                    }}
                  />
                  {open ? (
                    <ul className="harbor-code-pane-rows" aria-label={`${row.title ?? row.folder} panes`}>
                      {leaves.map((id, index) => (
                        <li key={id}>
                          <button
                            type="button"
                            className="harbor-code-pane-row"
                            aria-label={`${paneRowLabel(id)} pane ${index + 1}`}
                            data-selected={focused === id}
                            onClick={() => setFocused(id)}
                          >
                            <span className="harbor-pane-row-icon">
                              {paneKind(id, panes) === "terminal" ? (
                                <EngineMark
                                  engineId={panes.find((pane) => pane.id === id)?.engineId}
                                  label={terminalPaneLabel(id)}
                                  size={13}
                                />
                              ) : (
                                <PaneIcon kind={paneRowIconKind(id)} />
                              )}
                            </span>
                            {paneRowLabel(id)}
                          </button>
                        </li>
                      ))}
                      <li className="harbor-code-pane-add" ref={paneAddRef}>
                        <button
                          type="button"
                          className="harbor-code-pane-row harbor-code-pane-add-row"
                          aria-haspopup="menu"
                          aria-expanded={paneAddOpen}
                          disabled={!tabId}
                          onClick={() => setPaneAddOpen((value) => !value)}
                        >
                          <span className="harbor-pane-row-icon" aria-hidden="true">+</span>
                          New pane
                        </button>
                        {paneAddOpen ? (
                          <div className="harbor-pane-menu" role="menu">
                            <p className="harbor-pane-menu-heading">Terminal</p>
                            {terminalStarters.map((starter) => (
                              <button
                                key={starter.engineId}
                                type="button"
                                role="menuitem"
                                onClick={() => {
                                  setPaneAddOpen(false);
                                  void createPane("terminal", undefined, starter.engineId);
                                }}
                              >
                                <EngineMark engineId={starter.engineId} label={starter.label} size={13} />
                                <span>{starter.label}</span>
                              </button>
                            ))}
                            <p className="harbor-pane-menu-heading">Pane</p>
                            {(["files", "browser", "thread"] as CodePaneKind[]).map((kind) => (
                              <button
                                key={kind}
                                type="button"
                                role="menuitem"
                                onClick={() => {
                                  setPaneAddOpen(false);
                                  void createPane(kind);
                                }}
                              >
                                <PaneIcon kind={kind} />
                                <span>{paneLabel(kind)}</span>
                              </button>
                            ))}
                          </div>
                        ) : null}
                      </li>
                    </ul>
                  ) : null}
                </div>
              );
            }) : <p className="harbor-rail-empty">Add a folder to start coding.</p>}
          </div>
      </AppRail>
      <div className="harbor-stage-panel harbor-code">
        <div className="harbor-code-panes" ref={panesRootRef} style={{ gridTemplateColumns: "1fr" }}>
          {renderLayout(layout, (next) => {
            ++layoutActionRef.current;
            setLayoutError(null);
            setLayout(next);
          })}
        </div>
        {layoutError ? <p className="harbor-code-layout-error" role="alert">{layoutError}</p> : null}
      </div>
      {addingWorkspace ? <AddWorkspace onClose={() => setAddingWorkspace(false)} onAdded={(added) => {
        setAddingWorkspace(false);
        setWorkspaces((current) => current.some((item) => item.id === added.id) ? current : [...current, added]);
        void openWorkspace(added);
      }} /> : null}
    </div>
  );
}
