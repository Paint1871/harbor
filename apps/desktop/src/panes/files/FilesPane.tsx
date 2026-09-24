import { useEffect, useState } from "react";
import { call } from "../../ipc";
import { Tree } from "./Tree";
import { Editor } from "./Editor";
import { TabBar } from "./TabBar";
import { PaneHeader } from "../PaneHeader";
import { confirmCloseDirtyTab } from "./helpers";
import { discardDirtySource, hasUnsavedBuffers, saveUnsavedBuffers } from "./dirtyFiles";

const PANE_CLOSE_SAVE_FAILED = "Some files could not be saved. Close anyway?";

interface FilesPaneProps {
  workspaceId?: string;
  focused: boolean;
  onFocus: () => void;
  expanded?: boolean;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

export function FilesPane({ workspaceId: givenId, focused, onFocus, expanded = false, onExpand, onSplit, onClose }: FilesPaneProps) {
  const [fallbackWorkspaceId, setFallbackWorkspaceId] = useState<string | undefined>(givenId);
  useEffect(() => {
    // Code mode supplies the active workspace as soon as its native list is
    // ready. Avoid a second native workspace_list call during that hand-off:
    // competing scope grants can make the first file-tree read look like a
    // missing folder on a cold launch. The fallback is useful only for the
    // browser/preview host where FilesPane can be rendered in isolation.
    if (givenId || (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window)) return;
    void call("workspace_list")
      .then((list) => setFallbackWorkspaceId(list[0]?.id))
      .catch(() => undefined);
  }, [givenId]);
  const workspaceId = givenId ?? fallbackWorkspaceId;
  const [open, setOpen] = useState<string[]>([]);
  const [active, setActive] = useState<string | undefined>();
  const [dirty, setDirty] = useState<Record<string, boolean>>({});
  const [width, setWidth] = useState(220);

  function setPathDirty(path: string, isDirty: boolean) {
    setDirty((current) => {
      if (Boolean(current[path]) === isDirty) return current;
      if (!isDirty) {
        const next = { ...current };
        delete next[path];
        return next;
      }
      return { ...current, [path]: true };
    });
  }

  function closeTab(path: string) {
    if (!confirmCloseDirtyTab(Boolean(dirty[path]), (message) => window.confirm(message))) return;
    // A confirmed discard must reach the registry before the editor unmounts,
    // or its unmount flush would write the very changes the user dropped.
    if (dirty[path] && workspaceId) discardDirtySource(workspaceId, path);
    const index = open.indexOf(path);
    const next = open.filter((item) => item !== path);
    setOpen(next);
    setPathDirty(path, false);
    if (active === path) {
      setActive(next[Math.max(0, index - 1)]);
    }
  }

  function requestClose() {
    if (!onClose) return;
    if (!hasUnsavedBuffers()) {
      onClose();
      return;
    }
    // Flush dirty buffers first; only if a write genuinely fails does the
    // user decide whether to lose it.
    void saveUnsavedBuffers(2000).then((failed) => {
      if (!failed.length || window.confirm(PANE_CLOSE_SAVE_FAILED)) onClose();
    });
  }

  return (
    <section className="harbor-pane harbor-files" data-focused={focused} onClick={onFocus} aria-label="Files">
      <PaneHeader title="Files" live={focused} expanded={expanded} onExpand={onExpand} onSplit={onSplit} onClose={requestClose} />
      <div className="harbor-files-body">
        <div className="harbor-files-tree" style={{ width }}>
          <Tree
            key={workspaceId ?? "no-workspace"}
            workspaceId={workspaceId}
            onOpen={(path) => {
              setOpen((current) => (current.includes(path) ? current : [...current, path]));
              setActive(path);
            }}
          />
          <input
            aria-label="Tree width"
            type="range"
            min={140}
            max={360}
            value={width}
            onChange={(event) => setWidth(Number(event.target.value))}
          />
        </div>
        <div className="harbor-files-editor">
          <TabBar
            files={open}
            active={active}
            dirty={Object.keys(dirty).filter((path) => dirty[path])}
            onSelect={setActive}
            onClose={closeTab}
          />
          <Editor path={active} workspaceId={workspaceId} onDirtyChange={setPathDirty} />
        </div>
      </div>
    </section>
  );
}
