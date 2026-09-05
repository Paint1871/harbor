import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Workspace } from "@harbor/schema/commands";
import { Tree } from "./Tree";
import { Editor } from "./Editor";
import { TabBar } from "./TabBar";

interface FilesPaneProps {
  focused: boolean;
  onFocus: () => void;
}

export function FilesPane({ focused, onFocus }: FilesPaneProps) {
  const [workspaceId, setWorkspaceId] = useState<string | undefined>();
  useEffect(() => {
    void invoke<Workspace[]>("workspace_list")
      .then((list) => setWorkspaceId(list[0]?.id))
      .catch(() => undefined);
  }, []);
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
      <header>Files</header>
      <div className="harbor-files-body">
        <div className="harbor-files-tree" style={{ width }}>
          <Tree
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
