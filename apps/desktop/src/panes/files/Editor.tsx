import { useEffect, useRef, useState } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { filesystemError, readWorkspaceFile, writeWorkspaceFile } from "./fs";

interface EditorProps {
  path?: string;
  workspaceId?: string;
}

function languageFor(path: string) {
  if (path.endsWith(".json")) return json();
  if (path.endsWith(".md")) return markdown();
  return javascript();
}

export function Editor({ path, workspaceId }: EditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "saved" | "dirty" | "saving" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    if (!path || !workspaceId) {
      setState("idle");
      setError(null);
      return;
    }
    let cancelled = false;
    let revision = 0;
    setState("loading");
    setError(null);

    const save = (current: EditorView) => {
      const targetRevision = revision;
      setState("saving");
      void writeWorkspaceFile(workspaceId, path, current.state.doc.toString())
        .then(() => {
          if (cancelled || revision !== targetRevision) return;
          setState("saved");
          setError(null);
        })
        .catch((reason) => {
          if (cancelled) return;
          setState("error");
          setError(filesystemError(reason, "The file could not be saved."));
        });
    };

    void readWorkspaceFile(workspaceId, path)
      .then((doc) => {
        if (cancelled || !host.current) return;
        view.current?.destroy();
        view.current = new EditorView({
          state: EditorState.create({
            doc,
            extensions: [
              history(),
              keymap.of([
                ...defaultKeymap,
                ...historyKeymap,
                { key: "Mod-s", run: (current) => { save(current); return true; } },
              ]),
              languageFor(path),
              EditorView.updateListener.of((update) => {
                if (update.docChanged) {
                  revision += 1;
                  setState("dirty");
                }
              }),
              EditorView.theme({
                "&": { backgroundColor: "#0B0B0C", color: "#F5F5F5", height: "100%" },
                ".cm-content": { fontFamily: "ui-monospace, Menlo, monospace", fontSize: "13px" },
              }),
              EditorView.domEventHandlers({
                blur: (_event, current) => {
                  if (revision > 0) save(current);
                  return false;
                },
              }),
            ],
          }),
          parent: host.current,
        });
        setState("saved");
      })
      .catch((reason) => {
        if (cancelled) return;
        view.current?.destroy();
        view.current = null;
        setState("error");
        setError(filesystemError(reason, "The file could not be opened."));
      });
    return () => {
      cancelled = true;
      view.current?.destroy();
      view.current = null;
    };
  }, [path, workspaceId, reloadKey]);

  if (!path) return <div className="harbor-file-empty"><span className="harbor-eyebrow">EDITOR</span><strong>Open a file</strong><p>Select a file from the tree to inspect it here.</p></div>;
  const statusLabel = state === "saving" ? "Saving…" : state === "dirty" ? "Unsaved changes" : state === "error" ? "Needs attention" : "Saved";
  return <div className="harbor-editor-shell">
    <div className="harbor-editor-toolbar"><span title={path}>{path.split(/[\\/]/).pop() ?? path}</span><small data-state={state}>{statusLabel}</small></div>
    <div className="harbor-editor" ref={host} aria-label={path} />
    {state === "loading" ? <div className="harbor-file-overlay" role="status"><span className="harbor-spinner" aria-hidden="true" /><strong>Opening file…</strong><p>{path}</p></div> : null}
    {state === "error" ? <div className="harbor-file-overlay harbor-file-error" role="alert"><span className="harbor-eyebrow">EDITOR</span><strong>Could not open this file</strong><p>{error ?? "The file is unavailable."}</p><button type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button></div> : null}
  </div>;
}
