import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Terminal as XtermTerminal } from "@xterm/xterm";
import { Button } from "@harbor/ui/Button";
import { PaneHeader } from "./PaneHeader";
import "@xterm/xterm/css/xterm.css";

interface TerminalPaneProps {
  paneId?: string;
  focused: boolean;
  paused: boolean;
  onFocus: () => void;
  onResume: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

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

export function TerminalPane({
  paneId = "term",
  focused,
  paused,
  onFocus,
  onResume,
  onSplit,
  onClose,
}: TerminalPaneProps) {
  const host = useRef<HTMLDivElement>(null);
  const termRef = useRef<XtermTerminal | null>(null);

  useEffect(() => {
    if (!host.current || paused) return;
    if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
    let cancelled = false;
    let write: { dispose: () => void } | undefined;
    let stopListen: (() => void) | undefined;
    let onResize: (() => void) | undefined;
    void Promise.all([import("@xterm/xterm"), import("@xterm/addon-fit")]).then(([xterm, addon]) => {
      if (cancelled || !host.current) return;
      const term = new xterm.Terminal({
        cursorStyle: "bar",
        cursorBlink: false,
        fontFamily: "ui-monospace, Menlo, monospace",
        fontSize: 13,
        theme: { background: "#0B0B0C", foreground: "#F5F5F5" },
      });
      const fit = new addon.FitAddon();
      term.loadAddon(fit);
      term.open(host.current);
      fit.fit();
      termRef.current = term;
      void invoke<{ folder: string }[]>("workspace_list")
        .then((list) => list[0]?.folder ?? ".")
        .catch(() => ".")
        .then((cwd) => invoke("pty_spawn", { paneId, cwd, shell: null }))
        .catch((error) => {
          term.writeln(String(error));
        });
      write = term.onData((data) => {
        void invoke("pty_write_b64", { paneId, b64: bytesToB64(new TextEncoder().encode(data)) });
      });
      void listen<{ paneId: string; b64: string }>("pty-data", (event) => {
        if (event.payload.paneId !== paneId) return;
        term.write(b64ToBytes(event.payload.b64));
      }).then((stop) => {
        stopListen = stop;
      });
      onResize = () => {
        fit.fit();
        void invoke("pty_resize", { paneId, cols: term.cols, rows: term.rows });
      };
      window.addEventListener("resize", onResize);
    });
    return () => {
      cancelled = true;
      write?.dispose();
      if (onResize) window.removeEventListener("resize", onResize);
      stopListen?.();
      void invoke("pty_kill", { paneId });
      termRef.current?.dispose();
      termRef.current = null;
    };
  }, [paneId, paused]);

  return (
    <section className="harbor-pane" data-focused={focused} onClick={onFocus} aria-label="Terminal">
      <PaneHeader
        title="Terminal"
        live={!paused && focused}
        extra={paused ? <Button onClick={onResume}>Resume</Button> : null}
        onSplit={onSplit}
        onClose={onClose}
      />
      {paused ? <pre className="harbor-xterm">Restored terminal is paused.</pre> : <div className="harbor-xterm" ref={host} />}
    </section>
  );
}
