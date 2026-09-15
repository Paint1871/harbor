import { useEffect, useRef, useState } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { xml } from "@codemirror/lang-xml";
import { StreamLanguage } from "@codemirror/language";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { search, searchKeymap } from "@codemirror/search";
import { filesystemError, readWorkspaceFile, writeWorkspaceFile } from "./fs";
import { fileBasename, fileExtension, languageIdFor } from "./helpers";

interface EditorProps {
  path?: string;
  workspaceId?: string;
  onDirtyChange?: (path: string, dirty: boolean) => void;
}

function languageSupportFor(path: string) {
  const ext = fileExtension(path);
  switch (languageIdFor(path)) {
    case "python":
      return python();
    case "rust":
      return rust();
    case "css":
      return css();
    case "html":
      return html();
    case "xml":
      return xml();
    case "json":
      return json();
    case "markdown":
      return markdown();
    case "toml":
      return StreamLanguage.define(toml);
    case "javascript":
      return javascript({ jsx: ext === ".jsx" });
    case "typescript":
      return javascript({ typescript: true, jsx: ext === ".tsx" });
    default:
      return [];
  }
}

function themeIsDark(): boolean {
  return typeof document === "undefined" || document.documentElement.dataset.theme !== "light";
}

function harborEditorTheme() {
  return EditorView.theme({
    "&": { backgroundColor: "var(--harbor-bg)", color: "var(--harbor-text)", height: "100%" },
    ".cm-scroller": { fontFamily: "var(--harbor-font-mono)" },
    ".cm-content": { fontFamily: "var(--harbor-font-mono)", fontSize: "13px", caretColor: "var(--harbor-text)" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--harbor-text)" },
    ".cm-gutters": {
      backgroundColor: "var(--harbor-field)",
      color: "var(--harbor-text)",
      borderRight: "1px solid var(--harbor-border)",
    },
    ".cm-activeLine": { backgroundColor: "color-mix(in srgb, var(--harbor-text) 6%, transparent)" },
    ".cm-activeLineGutter": { backgroundColor: "color-mix(in srgb, var(--harbor-text) 6%, transparent)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
      background: "color-mix(in srgb, var(--harbor-text) 18%, transparent)",
    },
    ".cm-panels": { backgroundColor: "var(--harbor-field)", color: "var(--harbor-text)" },
    ".cm-panels-bottom": { borderTop: "1px solid var(--harbor-border)" },
    ".cm-panels-top": { borderBottom: "1px solid var(--harbor-border)" },
    ".cm-textfield": {
      backgroundColor: "var(--harbor-bg)",
      color: "var(--harbor-text)",
      border: "1px solid var(--harbor-border)",
    },
    ".cm-button": {
      background: "var(--harbor-field)",
      color: "var(--harbor-text)",
      border: "1px solid var(--harbor-border)",
    },
  }, { dark: themeIsDark() });
}

export function Editor({ path, workspaceId, onDirtyChange }: EditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const themeCompartment = useRef(new Compartment());
  const onDirtyChangeRef = useRef(onDirtyChange);
  onDirtyChangeRef.current = onDirtyChange;
  const [state, setState] = useState<"idle" | "loading" | "saved" | "dirty" | "saving" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    const root = document.documentElement;
    const observer = new MutationObserver(() => {
      const current = view.current;
      if (!current) return;
      current.dispatch({ effects: themeCompartment.current.reconfigure(harborEditorTheme()) });
    });
    observer.observe(root, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);

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
          onDirtyChangeRef.current?.(path, false);
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
              lineNumbers(),
              highlightActiveLineGutter(),
              highlightActiveLine(),
              drawSelection(),
              search(),
              keymap.of([
                ...searchKeymap,
                ...defaultKeymap,
                ...historyKeymap,
                { key: "Mod-s", run: (current) => { save(current); return true; } },
              ]),
              languageSupportFor(path),
              themeCompartment.current.of(harborEditorTheme()),
              EditorView.updateListener.of((update) => {
                if (update.docChanged) {
                  revision += 1;
                  setState("dirty");
                  onDirtyChangeRef.current?.(path, true);
                }
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
        onDirtyChangeRef.current?.(path, false);
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
      onDirtyChangeRef.current?.(path, false);
    };
  }, [path, workspaceId, reloadKey]);

  if (!path) return <div className="harbor-file-empty"><span className="harbor-eyebrow">EDITOR</span><strong>Open a file</strong><p>Select a file from the tree to inspect it here.</p></div>;
  const statusLabel = state === "saving" ? "Saving…" : state === "dirty" ? "Unsaved changes" : state === "error" ? "Needs attention" : "Saved";
  return <div className="harbor-editor-shell">
    <div className="harbor-editor-toolbar"><span title={path}>{fileBasename(path)}</span><small data-state={state}>{statusLabel}</small></div>
    <div className="harbor-editor" ref={host} aria-label={path} />
    {state === "loading" ? <div className="harbor-file-overlay" role="status"><span className="harbor-spinner" aria-hidden="true" /><strong>Opening file…</strong><p>{path}</p></div> : null}
    {state === "error" ? <div className="harbor-file-overlay harbor-file-error" role="alert"><span className="harbor-eyebrow">EDITOR</span><strong>Could not open this file</strong><p>{error ?? "The file is unavailable."}</p><button type="button" onClick={() => setReloadKey((value) => value + 1)}>Try again</button></div> : null}
  </div>;
}
