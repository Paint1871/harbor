import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";

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

  useEffect(() => {
    if (!host.current || !path || !workspaceId) return;
    let cancelled = false;
    void invoke<string>("fs_read", { workspaceId, path })
      .catch(() => "")
      .then((doc) => {
        if (cancelled || !host.current) return;
        view.current?.destroy();
        view.current = new EditorView({
          state: EditorState.create({
            doc,
            extensions: [
              history(),
              keymap.of([...defaultKeymap, ...historyKeymap]),
              languageFor(path),
              EditorView.theme({
                "&": { backgroundColor: "#0B0B0C", color: "#F5F5F5", height: "100%" },
                ".cm-content": { fontFamily: "ui-monospace, Menlo, monospace", fontSize: "13px" },
              }),
              EditorView.domEventHandlers({
                blur: (_event, current) => {
                  void invoke("fs_write", {
                    workspaceId,
                    path,
                    contents: current.state.doc.toString(),
                  }).catch(() => undefined);
                  return false;
                },
              }),
            ],
          }),
          parent: host.current,
        });
      });
    return () => {
      cancelled = true;
      view.current?.destroy();
      view.current = null;
    };
  }, [path, workspaceId]);

  if (!path) return <p className="harbor-muted">Open a file.</p>;
  return <div className="harbor-editor" ref={host} aria-label={path} />;
}
