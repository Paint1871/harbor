import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Composer } from "@harbor/ui/Composer";
import type { ThreadRecord, Workspace } from "@harbor/schema/commands";
import { FolderRail } from "./FolderRail";
import { ThreadHeader } from "./ThreadHeader";
import { ThreadList } from "./ThreadList";
import { ChangesPanel } from "./ChangesPanel";
import { PermissionCard } from "./PermissionCard";
import { ModelMenu } from "./ModelMenu";
import { useAcpThread } from "./useAcpThread";

export function ChatMode() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [threads, setThreads] = useState<ThreadRecord[]>([]);
  const [other, setOther] = useState<ThreadRecord[]>([]);
  const [active, setActive] = useState<ThreadRecord | null>(null);
  const [draft, setDraft] = useState("");
  const [folderInput, setFolderInput] = useState("");
  const acp = useAcpThread(active?.id ?? null);

  const reload = useCallback(async () => {
    try {
      const listed = await invoke<Workspace[]>("workspace_list");
      setWorkspaces(listed);
      const rest = await invoke<ThreadRecord[]>("thread_list", { workspaceId: null });
      setOther(rest);
      if (workspaceId) {
        setThreads(await invoke<ThreadRecord[]>("thread_list", { workspaceId }));
      } else {
        setThreads([]);
      }
    } catch {
      /* preview outside Tauri */
    }
  }, [workspaceId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function addWorkspace() {
    if (!folderInput.trim()) return;
    try {
      await invoke("workspace_add", { folder: folderInput.trim() });
      setFolderInput("");
      await reload();
    } catch {
      /* preview */
    }
  }

  async function newThread() {
    try {
      const thread = await invoke<ThreadRecord>("thread_create", {
        workspaceId,
        engineId: "opencode",
      });
      setActive(thread);
      await reload();
    } catch {
      /* preview */
    }
  }

  return (
    <div className="harbor-chat">
      <FolderRail
        workspaces={workspaces}
        otherCount={other.length}
        selectedId={workspaceId}
        folderInput={folderInput}
        onFolderInput={setFolderInput}
        onAddWorkspace={() => void addWorkspace()}
        onSelect={(id) => {
          setWorkspaceId(id);
          setActive(null);
        }}
      >
        <ThreadList
          threads={workspaceId ? threads : other}
          activeId={active?.id ?? null}
          onSelect={setActive}
          onPin={(id, pinned) => void invoke("thread_pin", { id, pinned }).then(reload)}
        />
      </FolderRail>
      <div className="harbor-chat-main">
        <ThreadHeader thread={active} workspace={workspaces.find((item) => item.id === workspaceId)} onNew={() => void newThread()} />
        <ChangesPanel workspaceId={workspaceId} refreshToken={acp.turn} />
        <div className="harbor-chat-transcript">
          {acp.lines.map((line) => (
            <p key={line.id}>{line.text}</p>
          ))}
          {acp.permission ? <PermissionCard request={acp.permission} onResolve={acp.resolve} /> : null}
        </div>
        <Composer
          value={draft}
          onValueChange={setDraft}
          onSend={(value) => {
            void acp.send(value);
            setDraft("");
          }}
          controls={
            <>
              <span className="harbor-muted">@ files only</span>
              <ModelMenu options={acp.configOptions} value={acp.model} onChange={acp.setModel} />
            </>
          }
        />
      </div>
    </div>
  );
}
