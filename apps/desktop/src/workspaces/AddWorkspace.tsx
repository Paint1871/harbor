import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine, Workspace, WorkspaceSetup } from "@harbor/schema/commands";
import { settingsGet, settingsSet } from "../settings";

const MAX_EXTRA_TERMINALS = 4;
const SHELL_ENGINE_ID = "shell";
const PREFERRED_ENGINE_IDS = ["claude-code", "opencode", "codex", "gemini", "copilot"];
const DEFAULT_SETUP: WorkspaceSetup = {
  additionalTerminals: 1,
  browserPreview: true,
  threadPane: true,
  terminalEngineIds: [],
};

function readSetup(value: unknown): WorkspaceSetup | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Partial<WorkspaceSetup>;
  if (
    typeof candidate.additionalTerminals !== "number"
    || typeof candidate.browserPreview !== "boolean"
    || typeof candidate.threadPane !== "boolean"
  ) {
    return null;
  }
  return {
    additionalTerminals: Math.min(MAX_EXTRA_TERMINALS, Math.max(0, Math.round(candidate.additionalTerminals))),
    browserPreview: candidate.browserPreview,
    threadPane: candidate.threadPane,
    terminalEngineIds: Array.isArray(candidate.terminalEngineIds)
      ? candidate.terminalEngineIds.filter((id): id is string => typeof id === "string")
      : [],
  };
}

function preferredEngineId(engines: DetectedEngine[]): string {
  const ready = new Set(readyEngines(engines).map((engine) => engine.id));
  return PREFERRED_ENGINE_IDS.find((id) => ready.has(id)) ?? readyEngines(engines)[0]?.id ?? SHELL_ENGINE_ID;
}

function readyEngines(engines: DetectedEngine[]): DetectedEngine[] {
  return engines.filter((engine) => engine.supportsTerminal !== false && engine.status !== "cli-missing" && Boolean(engine.path));
}

function normalizeEngineIds(ids: string[], total: number, engines: DetectedEngine[]): string[] {
  const available = new Set(readyEngines(engines).map((engine) => engine.id));
  const fallback = preferredEngineId(engines);
  return Array.from({ length: total }, (_, index) => {
    const id = ids[index];
    return id === SHELL_ENGINE_ID || (id && available.has(id)) ? id : fallback;
  });
}

function engineLabel(id: string, engines: DetectedEngine[]): string {
  if (id === SHELL_ENGINE_ID) return "Shell";
  return engines.find((engine) => engine.id === id)?.displayName ?? id;
}

function executableName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function AddWorkspace({
  onAdded,
  onClose,
}: {
  onAdded: (workspace: Workspace) => void;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const setupTouched = useRef(false);
  const [folder, setFolder] = useState("");
  const [additionalTerminals, setAdditionalTerminals] = useState(DEFAULT_SETUP.additionalTerminals);
  const [browserPreview, setBrowserPreview] = useState(DEFAULT_SETUP.browserPreview);
  const [threadPane, setThreadPane] = useState(DEFAULT_SETUP.threadPane);
  const [terminalEngineIds, setTerminalEngineIds] = useState(DEFAULT_SETUP.terminalEngineIds);
  // One CLI for the whole workspace is the common case; per-terminal is opt-in.
  const [perTerminal, setPerTerminal] = useState(false);
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [detectionState, setDetectionState] = useState<"checking" | "ready" | "error">("checking");
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const detectEngines = useCallback(async (command: "engines_detect" | "engines_recheck" = "engines_detect") => {
    setDetectionState("checking");
    try {
      const detected = await invoke<DetectedEngine[]>(command);
      const next = Array.isArray(detected) ? detected : [];
      setEngines(next);
      setDetectionState("ready");
    } catch {
      setEngines([]);
      setDetectionState("error");
    }
  }, []);

  useEffect(() => {
    let active = true;
    if (dialog.current && typeof dialog.current.showModal === "function") {
      dialog.current.showModal();
    }
    void settingsGet("workspace_launch_defaults")
      .then((value) => {
        const setup = readSetup(value);
        if (!active || setupTouched.current || !setup) return;
        setAdditionalTerminals(setup.additionalTerminals);
        setBrowserPreview(setup.browserPreview);
        setThreadPane(setup.threadPane);
        setTerminalEngineIds(setup.terminalEngineIds);
      })
      .catch(() => undefined)
      .finally(() => {
        if (active) setSettingsLoaded(true);
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    void detectEngines();
  }, [detectEngines]);

  const totalTerminals = additionalTerminals + 1;
  const availableEngines = readyEngines(engines);
  const defaultEngineId = terminalEngineIds[0] || preferredEngineId(engines);
  const defaultEngine = availableEngines.find((engine) => engine.id === defaultEngineId);

  useEffect(() => {
    if (detectionState !== "ready" || !settingsLoaded) return;
    setTerminalEngineIds((current) => normalizeEngineIds(current, totalTerminals, engines));
  }, [detectionState, engines, settingsLoaded, totalTerminals]);

  async function browse() {
    setBusy(true);
    setError(null);
    try {
      const path = await invoke<string | null>("workspace_pick_folder");
      if (path) setFolder(path);
    } catch {
      setError("The folder picker is unavailable. Paste the full folder path below.");
    } finally {
      setBusy(false);
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || !folder.trim() || detectionState === "checking") return;
    setBusy(true);
    setError(null);
    const setup: WorkspaceSetup = {
      additionalTerminals,
      browserPreview,
      threadPane,
      terminalEngineIds: terminalEngineIds.slice(0, totalTerminals).map((id) => id || SHELL_ENGINE_ID),
    };
    try {
      const added = await invoke<Workspace>("workspace_add", { folder: folder.trim() });
      await invoke("workspace_configure_tab", { workspaceId: added.id, setup });
      await settingsSet("workspace_launch_defaults", setup).catch(() => undefined);
      onAdded(added);
    } catch (reason) {
      setError("Could not prepare this workspace. " + String(reason).replace(/^Error:\s*/i, ""));
    } finally {
      setBusy(false);
    }
  }

  const visibleTerminalEngineIds = Array.from(
    { length: totalTerminals },
    (_, index) => terminalEngineIds[index] || (detectionState === "ready" ? preferredEngineId(engines) : SHELL_ENGINE_ID),
  );
  const startPanes = [
    `${totalTerminals} ${totalTerminals === 1 ? "terminal" : "terminals"} · ${visibleTerminalEngineIds
      .map((id) => engineLabel(id, engines))
      .join(" + ")}`,
    browserPreview ? "localhost:3000" : null,
    threadPane ? "Thread" : null,
  ].filter((item): item is string => Boolean(item));

  return (
    <dialog
      ref={dialog}
      className="harbor-dialog harbor-agent-dialog harbor-workspace-dialog"
      aria-labelledby="add-folder-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <form onSubmit={(event) => void submit(event)}>
        <span className="harbor-eyebrow">WORKSPACE FOLDER</span>
        <h2 id="add-folder-title">Bring your project into Harbor.</h2>
        <p>Your files stay in place. Harbor groups conversations under this folder.</p>

        <Button type="button" className="harbor-browse-folder" disabled={busy} onClick={() => void browse()}>
          Choose folder…
        </Button>
        <label>
          Or enter a full folder path
          <input
            autoFocus
            disabled={busy}
            value={folder}
            onChange={(event) => setFolder(event.target.value)}
            placeholder="Full path to your project"
          />
        </label>

        <fieldset className="harbor-workspace-setup">
          <legend>Start layout</legend>
          <label className="harbor-workspace-slider">
            <span className="harbor-workspace-slider-heading">
              <span>
                <strong>Additional terminals</strong>
                <small>Open more terminals beside the primary CLI.</small>
              </span>
              <output aria-live="polite">
                {additionalTerminals === 0 ? "Main only" : "+" + additionalTerminals}
              </output>
            </span>
            <input
              type="range"
              min="0"
              max={MAX_EXTRA_TERMINALS}
              step="1"
              value={additionalTerminals}
              aria-label="Additional terminals"
              onChange={(event) => {
                setupTouched.current = true;
                setAdditionalTerminals(Number(event.target.value));
              }}
            />
            <span className="harbor-workspace-slider-ticks" aria-hidden="true">
              <span>0</span><span>1</span><span>2</span><span>3</span><span>4</span>
            </span>
          </label>

          <div className="harbor-workspace-terminal-heading">
            <span>
              <strong>Terminal launch</strong>
              <small>Which installed CLI these terminals open with.</small>
            </span>
            <button
              type="button"
              className="harbor-workspace-detect"
              disabled={busy || detectionState === "checking"}
              onClick={() => void detectEngines("engines_recheck")}
            >
              {detectionState === "checking" ? "Checking…" : "Check again"}
            </button>
          </div>
          <div className="harbor-workspace-terminals" aria-live="polite">
            {!perTerminal ? (
              <label className="harbor-workspace-terminal">
                <span className="harbor-workspace-terminal-label">
                  <strong>All terminals</strong>
                  <small>{totalTerminals === 1 ? "1 terminal" : `${totalTerminals} terminals`}</small>
                </span>
                <span className="harbor-workspace-terminal-control">
                  <select
                    aria-label="Terminal CLI"
                    disabled={busy}
                    value={defaultEngineId}
                    onChange={(event) => {
                      setupTouched.current = true;
                      const next = event.target.value;
                      setTerminalEngineIds(Array.from({ length: totalTerminals }, () => next));
                    }}
                  >
                    <option value={SHELL_ENGINE_ID}>Shell only</option>
                    {availableEngines.map((engine) => (
                      <option value={engine.id} key={engine.id}>{engine.displayName}</option>
                    ))}
                  </select>
                  <small>
                    {defaultEngine
                      ? `${executableName(defaultEngine.path)} detected`
                      : "Uses your login shell"}
                  </small>
                </span>
              </label>
            ) : (
              Array.from({ length: totalTerminals }, (_, index) => {
              const selectedId = terminalEngineIds[index] || SHELL_ENGINE_ID;
              const selectedEngine = availableEngines.find((engine) => engine.id === selectedId);
              const missingSelection = selectedId !== SHELL_ENGINE_ID && !selectedEngine;
              return (
                <label className="harbor-workspace-terminal" key={index}>
                  <span className="harbor-workspace-terminal-label">
                    <strong>Terminal {index + 1}</strong>
                    <small>{index === 0 ? "Primary" : "Additional"}</small>
                  </span>
                  <span className="harbor-workspace-terminal-control">
                    <select
                      aria-label={`Terminal ${index + 1} CLI`}
                      disabled={busy}
                      value={selectedId}
                      onChange={(event) => {
                        setupTouched.current = true;
                        setTerminalEngineIds((current) => {
                          const next = normalizeEngineIds(current, totalTerminals, engines);
                          next[index] = event.target.value;
                          return next;
                        });
                      }}
                    >
                      <option value={SHELL_ENGINE_ID}>Shell only</option>
                      {missingSelection ? <option value={selectedId}>{selectedId} · unavailable</option> : null}
                      {availableEngines.map((engine) => (
                        <option value={engine.id} key={engine.id}>{engine.displayName}</option>
                      ))}
                    </select>
                    <small className={missingSelection ? "harbor-workspace-terminal-missing" : undefined}>
                      {selectedEngine
                        ? `${executableName(selectedEngine.path)} detected`
                        : selectedId === SHELL_ENGINE_ID
                          ? "Uses your login shell"
                          : detectionState === "checking"
                            ? "Checking this CLI…"
                            : "CLI unavailable · choose Shell or check again"}
                    </small>
                  </span>
                </label>
              );
              })
            )}
            {totalTerminals > 1 ? (
              <label className="harbor-workspace-per-terminal">
                <input
                  type="checkbox"
                  checked={perTerminal}
                  disabled={busy}
                  onChange={(event) => setPerTerminal(event.target.checked)}
                />
                <span>Give each terminal its own CLI</span>
              </label>
            ) : null}
          </div>
          {detectionState === "error" ? (
            <p className="harbor-workspace-detection-note">Could not inspect local CLIs. Harbor will still open regular shells.</p>
          ) : availableEngines.length === 0 && detectionState === "ready" ? (
            <p className="harbor-workspace-detection-note">No supported coding CLI detected. Install one or use Shell only.</p>
          ) : (
            <p className="harbor-workspace-detection-note">{availableEngines.length} installed CLI{availableEngines.length === 1 ? "" : "s"} detected on this Mac.</p>
          )}

          <div className="harbor-workspace-options">
            <label className="harbor-workspace-option">
              <input
                type="checkbox"
                checked={browserPreview}
                onChange={(event) => {
                  setupTouched.current = true;
                  setBrowserPreview(event.target.checked);
                }}
              />
              <span>
                <strong>Browser preview</strong>
                <small>Keep localhost:3000 ready in the right column.</small>
              </span>
            </label>
            <label className="harbor-workspace-option">
              <input
                type="checkbox"
                checked={threadPane}
                onChange={(event) => {
                  setupTouched.current = true;
                  setThreadPane(event.target.checked);
                }}
              />
              <span>
                <strong>Thread pane</strong>
                <small>Keep the task conversation visible beside your code.</small>
              </span>
            </label>
          </div>
          <p className="harbor-workspace-summary">
            Starts with {startPanes.join(" · ")}. You can add or close panes anytime.
          </p>
        </fieldset>

        {error ? <p role="alert" className="harbor-inline-error">{error}</p> : null}
        <div className="harbor-dialog-actions">
          <Button type="button" disabled={busy} onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="primary" disabled={busy || !folder.trim() || detectionState === "checking"}>
            {busy ? "Opening…" : detectionState === "checking" ? "Checking CLIs…" : "Open folder"}
          </Button>
        </div>
      </form>
    </dialog>
  );
}
