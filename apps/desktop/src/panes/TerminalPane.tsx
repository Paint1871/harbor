import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Terminal as XtermTerminal } from "@xterm/xterm";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine } from "@harbor/schema/commands";
import { PaneHeader } from "./PaneHeader";
import "@xterm/xterm/css/xterm.css";
import { EngineMark } from "../chrome/EngineMark";

interface TerminalPaneProps {
  paneId?: string;
  label?: string;
  engineId?: string | null;
  availableEngines?: DetectedEngine[];
  onEngineChange?: (engineId: string) => Promise<void> | void;
  workspaceId?: string;
  focused: boolean;
  paused: boolean;
  onFocus: () => void;
  onResume: () => void;
  onStatusChange?: (status: TerminalStatus) => void;
  expanded?: boolean;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

export type TerminalStatus = "waiting" | "starting" | "running" | "paused" | "error";

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
}

function b64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function errorText(reason: unknown): string {
  const text = String(reason).replace(/^Error:\s*/i, "").trim();
  return text || "The terminal could not start.";
}

export function TerminalPane({
  paneId = "term",
  label = "Terminal",
  engineId,
  availableEngines = [],
  onEngineChange,
  workspaceId,
  focused,
  paused,
  onFocus,
  onResume,
  onStatusChange,
  expanded = false,
  onExpand,
  onSplit,
  onClose,
}: TerminalPaneProps) {
  const host = useRef<HTMLDivElement>(null);
  const termRef = useRef<XtermTerminal | null>(null);
  const [status, setStatus] = useState<TerminalStatus>(paused ? "paused" : workspaceId ? "starting" : "waiting");
  const [error, setError] = useState<string | null>(null);
  const [restartKey, setRestartKey] = useState(0);
  const [engineChangeBusy, setEngineChangeBusy] = useState(false);
  const onStatusChangeRef = useRef(onStatusChange);
  onStatusChangeRef.current = onStatusChange;

  useEffect(() => {
    onStatusChangeRef.current?.(status);
  }, [status]);

  useEffect(() => {
    if (paused) {
      setStatus("paused");
      setError(null);
      return;
    }
    if (!workspaceId || !host.current) {
      setStatus("waiting");
      setError(null);
      return;
    }
    if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
      setStatus("waiting");
      setError(null);
      return;
    }

    let cancelled = false;
    let spawned = false;
    let exited = false;
    setError(null);
    let stopListen: (() => void) | undefined;
    let disposeInput: { dispose: () => void } | undefined;
    let resizeObserver: ResizeObserver | undefined;
    let onWindowResize: (() => void) | undefined;
    let term: XtermTerminal | undefined;

    const setup = async () => {
      const [xterm, addon] = await Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")]);
      if (cancelled || !host.current) return;

      term = new xterm.Terminal({
        cursorStyle: "bar",
        cursorBlink: true,
        convertEol: true,
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        fontSize: 13,
        fontWeight: "400",
        lineHeight: 1.25,
        scrollback: 5000,
        scrollSensitivity: 1,
        theme: {
          background: "#0A0A0C",
          foreground: "#E8E8EE",
          cursor: "#79A7FF",
          cursorAccent: "#0A0A0C",
          selectionBackground: "rgba(93, 142, 255, 0.32)",
          black: "#0A0A0C",
          red: "#FF7B8A",
          green: "#78D6A2",
          yellow: "#E7C77A",
          blue: "#79A7FF",
          magenta: "#C69CFF",
          cyan: "#74DCE8",
          white: "#E8E8EE",
          brightBlack: "#6D6D78",
          brightRed: "#FF9AA5",
          brightGreen: "#98E7B7",
          brightYellow: "#F0D99B",
          brightBlue: "#A5C2FF",
          brightMagenta: "#D8BBFF",
          brightCyan: "#A1EDF3",
          brightWhite: "#FFFFFF",
        },
      });
      const fit = new addon.FitAddon();
      term.loadAddon(fit);
      term.open(host.current);
      termRef.current = term;

      const resize = () => {
        if (cancelled || !term) return;
        fit.fit();
        if (!spawned) return;
        void invoke("pty_resize", { paneId, cols: term.cols, rows: term.rows }).catch((reason) => {
          if (!cancelled) setError(errorText(reason));
        });
      };
      resize();
      onWindowResize = resize;
      window.addEventListener("resize", resize);
      if (typeof ResizeObserver !== "undefined") {
        resizeObserver = new ResizeObserver(resize);
        resizeObserver.observe(host.current);
      }

      disposeInput = term.onData((data) => {
        if (!spawned) return;
        void invoke("pty_write_b64", {
          paneId,
          b64: bytesToB64(new TextEncoder().encode(data)),
        }).catch((reason) => {
          if (!cancelled) {
            setStatus("error");
            setError(errorText(reason));
          }
        });
      });

      const [stopData, stopExit] = await Promise.all([
        listen<{ paneId: string; b64: string }>("pty-data", (event) => {
          if (event.payload.paneId !== paneId || cancelled || !term) return;
          try {
            term.write(b64ToBytes(event.payload.b64));
          } catch (reason) {
            setStatus("error");
            setError(errorText(reason));
          }
        }),
        listen<{ paneId: string }>("pty-exit", (event) => {
          if (event.payload.paneId !== paneId || cancelled) return;
          exited = true;
          spawned = false;
          setStatus("error");
          setError("The terminal process exited unexpectedly. Retry to start a fresh terminal.");
        }),
      ]);
      if (cancelled) {
        stopData();
        stopExit();
        return;
      }
      stopListen = () => {
        stopData();
        stopExit();
      };

      setStatus("starting");
      await invoke("pty_spawn", {
        paneId,
        workspaceId,
        cols: term.cols,
        rows: term.rows,
        shell: null,
        engineId: engineId ?? null,
      });
      if (cancelled || exited) return;
      spawned = true;
      setStatus("running");
      resize();
    };

    void setup().catch((reason) => {
      if (cancelled) return;
      setStatus("error");
      setError(errorText(reason));
      term?.writeln(`\r\nHarbor could not start this terminal.\r\n${errorText(reason)}\r\n`);
    });

    return () => {
      cancelled = true;
      spawned = false;
      disposeInput?.dispose();
      stopListen?.();
      resizeObserver?.disconnect();
      if (onWindowResize) window.removeEventListener("resize", onWindowResize);
      void invoke("pty_kill", { paneId }).catch(() => undefined);
      term?.dispose();
      termRef.current = null;
    };
  }, [engineId, paneId, paused, restartKey, workspaceId]);

  const statusLabel = {
    waiting: "Waiting for workspace",
    starting: "Starting",
    running: "Running",
    paused: "Paused",
    error: "Needs attention",
  }[status];
  const terminalEngines = availableEngines.filter((engine) => engine.supportsTerminal !== false && engine.status !== "cli-missing" && Boolean(engine.path));
  const selectedEngineId = engineId && engineId !== "" ? engineId : "shell";
  const selectedEngineIsAvailable = selectedEngineId === "shell" || terminalEngines.some((engine) => engine.id === selectedEngineId);

  return (
    <section
      className="harbor-pane harbor-terminal-pane"
      data-focused={focused}
      data-status={status}
      onClick={onFocus}
      aria-label={`${label} ${statusLabel}`}
    >
      <PaneHeader
        title={label}
        live={status === "running" && focused}
        leading={<EngineMark engineId={engineId} label={label} />}
        expanded={expanded}
        onExpand={onExpand}
        extra={
          <>
            {status !== "running" ? <span className="harbor-terminal-status" data-status={status}>{statusLabel}</span> : null}
            {status === "paused" ? <Button onClick={onResume}>Resume</Button> : null}
            {status === "error" ? <Button onClick={() => setRestartKey((value) => value + 1)}>Retry</Button> : null}
          </>
        }
        menuContent={onEngineChange ? (
          <label className="harbor-pane-menu-field">
            <span>Terminal CLI</span>
            <select
              aria-label={`CLI for ${label}`}
              value={selectedEngineIsAvailable ? selectedEngineId : "__missing__"}
              disabled={engineChangeBusy || status === "starting"}
              onChange={(event) => {
                const next = event.target.value;
                if (next === "__missing__") return;
                setEngineChangeBusy(true);
                void Promise.resolve()
                  .then(() => onEngineChange(next))
                  .catch((reason) => setError(errorText(reason)))
                  .finally(() => setEngineChangeBusy(false));
              }}
            >
              {!selectedEngineIsAvailable ? <option value="__missing__">{label} unavailable</option> : null}
              <option value="shell">Shell</option>
              {terminalEngines.map((engine) => <option key={engine.id} value={engine.id}>{engine.displayName}</option>)}
            </select>
            <small>{engineChangeBusy ? "Restarting terminal…" : "Changes apply to this pane."}</small>
          </label>
        ) : null}
        onClear={status === "running" ? () => {
          termRef.current?.clear();
          termRef.current?.focus();
        } : undefined}
        onSplit={onSplit}
        onClose={onClose}
      />
      {paused ? (
        <div className="harbor-terminal-state" role="status">
          <span className="harbor-terminal-state-mark" aria-hidden="true">Ⅱ</span>
          <strong>Terminal paused</strong>
          <p>The previous session is closed safely. Resume to start a fresh shell in this workspace.</p>
          <Button variant="primary" onClick={onResume}>Resume terminal</Button>
        </div>
      ) : !workspaceId ? (
        <div className="harbor-terminal-state" role="status">
          <span className="harbor-terminal-state-mark" aria-hidden="true">…</span>
          <strong>Waiting for a workspace</strong>
          <p>Open a folder from the workspace rail to start a terminal here.</p>
        </div>
      ) : (
        <div className="harbor-xterm" ref={host} />
      )}
      {error ? <p className="harbor-terminal-error" role="alert">{error}</p> : null}
    </section>
  );
}
