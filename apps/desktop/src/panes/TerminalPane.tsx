import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Button } from "@harbor/ui/Button";
import "@xterm/xterm/css/xterm.css";

interface TerminalPaneProps {
  paneId?: string;
  focused: boolean;
  paused: boolean;
  onFocus: () => void;
  onResume: () => void;
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

export function TerminalPane({ paneId = "term", focused, paused, onFocus, onResume }: TerminalPaneProps) {
  const host = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!host.current || paused) return;
    const term = new Terminal({
      cursorStyle: "bar",
      cursorBlink: false,
      fontFamily: "ui-monospace, Menlo, monospace",
      fontSize: 13,
      theme: { background: "#0B0B0C", foreground: "#F5F5F5" },
    });
    const fit = new FitAddon();
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
    const write = term.onData((data) => {
      void invoke("pty_write_b64", { paneId, b64: bytesToB64(new TextEncoder().encode(data)) });
    });
    const unlisten = listen<{ paneId: string; b64: string }>("pty-data", (event) => {
      if (event.payload.paneId !== paneId) return;
      term.write(b64ToBytes(event.payload.b64));
    });
    const onResize = () => {
      fit.fit();
      void invoke("pty_resize", { paneId, cols: term.cols, rows: term.rows });
    };
    window.addEventListener("resize", onResize);
    return () => {
      write.dispose();
      window.removeEventListener("resize", onResize);
      void unlisten.then((stop) => stop());
      void invoke("pty_kill", { paneId });
      term.dispose();
      termRef.current = null;
    };
  }, [paneId, paused]);

  return (
    <section className="harbor-pane" data-focused={focused} onClick={onFocus} aria-label="Terminal">
      <header>
        <span className="harbor-live-dot" data-on={!paused && focused} />
        Terminal
        {paused ? <Button onClick={onResume}>Resume</Button> : null}
      </header>
      {paused ? <pre className="harbor-xterm">Restored terminal is paused.</pre> : <div className="harbor-xterm" ref={host} />}
    </section>
  );
}
