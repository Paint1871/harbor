import { useEffect, useState } from "react";
import { call } from "../../ipc";
import { Tree } from "./Tree";
import { Editor } from "./Editor";
import { TabBar } from "./TabBar";
import { PaneHeader } from "../PaneHeader";

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
  const [width, setWidth] = useState(220);

  function closeTab(path: string) {
    const index = open.indexOf(path);
    const next = open.filter((item) => item !== path);
    setOpen(next);
    if (active === path) {
      setActive(next[Math.max(0, index - 1)]);
    }
  }

  return (
    <section className="harbor-pane harbor-files" data-focused={focused} onClick={onFocus} aria-label="Files">
      <PaneHeader title="Files" live={focused} expanded={expanded} onExpand={onExpand} onSplit={onSplit} onClose={onClose} />
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
          <TabBar files={open} active={active} onSelect={setActive} onClose={closeTab} />
          <Editor path={active} workspaceId={workspaceId} />
        </div>
      </div>
    </section>
  );
}
