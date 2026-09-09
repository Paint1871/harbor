import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../../ipc";
import { Composer } from "@harbor/ui/Composer";
import { Button } from "@harbor/ui/Button";
import { Logo } from "@harbor/ui/Logo";
import type { DetectedEngine, FsEntry, ThreadRecord, Workspace } from "@harbor/schema/commands";
import { MentionList, mentionQuery } from "../../chrome/MentionList";
import { FolderRail } from "./FolderRail";
import { ThreadHeader } from "./ThreadHeader";
import { ThreadList } from "./ThreadList";
import { ChangesPanel } from "./ChangesPanel";
import { EngineControls } from "./EngineControls";
import { ContextMeter } from "./ContextMeter";
import { PermissionCard } from "./PermissionCard";
import { useAcpThread } from "./useAcpThread";
import { AppRail } from "../../chrome/AppRail";
import { Transcript } from "../../chrome/Transcript";
import { useChrome } from "../../chrome/chrome-context";
import { AddWorkspace } from "../../workspaces/AddWorkspace";

export function ChatMode({ railOpen = true }: { railOpen?: boolean }) {
  const { mode, setDestination } = useChrome();
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [lists, setLists] = useState<Record<string, ThreadRecord[]>>({});
  const [other, setOther] = useState<ThreadRecord[]>([]);
  const [active, setActive] = useState<ThreadRecord | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [adding, setAdding] = useState(false);
  const [creating, setCreating] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [engineId, setEngineId] = useState("");
  const [checking, setChecking] = useState(false);
  const [configChoice, setConfigChoice] = useState<Record<string, Record<string, string>>>({});
  const [attached, setAttached] = useState<Record<string, string[]>>({});
  const [fileMentions, setFileMentions] = useState<FsEntry[]>([]);
  const [optionsBusy, setOptionsBusy] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const createLock = useRef(false);
  const acp = useAcpThread(active?.id ?? null);
  const workspace = workspaces.find((item) => item.id === workspaceId);
  const ready = engines.filter((engine) => engine.status === "ready" && engine.supportsChat);
  // Chat-capable engines that are installed but still need their ACP adapter.
  const installable = engines.filter((engine) => engine.status === "adapter-missing" && !!engine.adapterPackage);
  const engine = ready.find((item) => item.id === engineId) ?? ready[0];
  const threads = workspaceId ? lists[workspaceId] : other;

  const checkEngines = useCallback(async () => {
    setChecking(true);
    try { setEngines(await call("engines_detect")); }
    catch { setError("Could not check installed engines. Try again in the desktop app."); }
    finally { setChecking(false); }
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [listed, rest] = await Promise.all([
        call("workspace_list"),
        call("thread_list", { workspaceId: null }),
      ]);
      setWorkspaces(listed);
      setOther(rest);
      setWorkspaceId((current) => current ?? (rest.length ? null : listed[0]?.id ?? null));
    } catch { setError("Your folders could not be loaded. Please try again."); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => {
    if (mode !== "chat") return;
    void reload();
    void checkEngines();
  }, [checkEngines, mode, reload]);

  useEffect(() => {
    if (!active || acp.configOptions.length) return;
    setOptionsBusy(true);
    void acp.loadConfigOptions().finally(() => setOptionsBusy(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active?.id]);

  // Tell the host which conversation is on screen, so a reply the builder is
  // watching land does not also arrive as an inbox row.
  useEffect(() => {
    const sessionRef = mode === "chat" ? active?.id ?? null : null;
    void call("session_watch", { sessionRef }).catch(() => undefined);
    return () => { void call("session_watch", { sessionRef: null }).catch(() => undefined); };
  }, [active?.id, mode]);

  useEffect(() => {
    if (!workspaceId) return;
    let disposed = false;
    void call("thread_list", { workspaceId })
      .then((items) => { if (!disposed) setLists((current) => ({ ...current, [workspaceId]: items })); })
      .catch(() => { if (!disposed) setError("This folder’s threads could not be loaded. Reopen the folder to try again."); });
    return () => { disposed = true; };
  }, [workspaceId, acp.turn]);

  async function newThread() {
    setDestination("mode");
    if (!workspace) { setAdding(true); return; }
    if (!engine || createLock.current) return;
    createLock.current = true;
    setCreating(true);
    setError(null);
    try {
      const thread = await call("thread_create", { workspaceId: workspace.id, engineId: engine.id });
      setLists((current) => ({ ...current, [workspace.id]: [thread, ...(current[workspace.id] ?? [])] }));
      setActive(thread);
    } catch { setError("The thread could not be created. Your folder is still open. Please try again."); }
    finally { setCreating(false); createLock.current = false; }
  }

  async function installAdapter(engineId: string) {
    setInstalling(engineId);
    setError(null);
    try {
      setEngines(await call("engine_install_adapter", { engineId }));
    } catch (reason) {
      setError(`Could not add that engine. ${String(reason)}`);
    } finally {
      setInstalling(null);
    }
  }

  async function setThreadEngine(threadId: string, nextEngineId: string) {
    try {
      await call("thread_set_engine", { id: threadId, engineId: nextEngineId });
      const apply = (items: ThreadRecord[]) => items.map((item) => item.id === threadId ? { ...item, engineId: nextEngineId } : item);
      if (workspaceId) setLists((current) => ({ ...current, [workspaceId]: apply(current[workspaceId] ?? []) }));
      else setOther(apply);
      setActive((current) => current?.id === threadId ? { ...current, engineId: nextEngineId } : current);
      // The old engine's models and option values died with its session. Show
      // the new engine's, not a Claude model list under a Grok pill.
      setConfigChoice((current) => {
        const { [threadId]: _dropped, ...rest } = current;
        return rest;
      });
      acp.resetConfigOptions();
      setOptionsBusy(true);
      void acp.loadConfigOptions().finally(() => setOptionsBusy(false));
    } catch (reason) {
      setError(`Could not switch the engine for this thread. ${String(reason)}`);
    }
  }

  async function pin(id: string, pinned: boolean) {
    try {
      await call("thread_pin", { id, pinned });
      const updated = (items: ThreadRecord[]) => items.map((item) => item.id === id ? { ...item, pinned } : item).sort((a, b) => Number(b.pinned) - Number(a.pinned));
      if (workspaceId) setLists((current) => ({ ...current, [workspaceId]: updated(current[workspaceId] ?? []) }));
      else setOther(updated);
    } catch { setError("Could not update the pin. Please try again."); }
  }

  async function renameThread(id: string, title: string) {
    try {
      await call("thread_rename", { id, title });
      const apply = (items: ThreadRecord[]) => items.map((item) => item.id === id ? { ...item, title } : item);
      if (workspaceId) setLists((current) => ({ ...current, [workspaceId]: apply(current[workspaceId] ?? []) }));
      else setOther(apply);
      setActive((current) => current?.id === id ? { ...current, title } : current);
    } catch { setError("Could not rename that thread. Please try again."); }
  }

  async function deleteThread(id: string) {
    try {
      await call("thread_delete", { id });
      const drop = (items: ThreadRecord[]) => items.filter((item) => item.id !== id);
      if (workspaceId) setLists((current) => ({ ...current, [workspaceId]: drop(current[workspaceId] ?? []) }));
      else setOther(drop);
      setActive((current) => current?.id === id ? null : current);
      // The thread is gone; its draft, attachments and option values go with it.
      setDrafts((current) => { const { [id]: _dropped, ...rest } = current; return rest; });
      setAttached((current) => { const { [id]: _dropped, ...rest } = current; return rest; });
      setConfigChoice((current) => { const { [id]: _dropped, ...rest } = current; return rest; });
    } catch { setError("Could not delete that thread. Please try again."); }
  }

  function filePath(file: File): string | null {
    if ("path" in file && typeof (file as { path?: unknown }).path === "string") {
      const path = (file as { path: string }).path.trim();
      return path || null;
    }
    return null;
  }

  async function attachFolder() {
    if (!active) return;
    try {
      const folder = await call("workspace_pick_folder");
      if (!folder) return;
      await call("thread_attach_files", { id: active.id, paths: [folder] });
      setAttached((current) => ({ ...current, [active.id]: [...(current[active.id] ?? []), folder] }));
    } catch (reason) {
      setError(`Could not attach that folder. ${String(reason)}`);
    }
  }

  async function attachFiles(list: FileList | null) {
    if (!active || !list?.length) return;
    const paths = Array.from(list).flatMap((file) => {
      const path = filePath(file);
      return path ? [path] : [];
    });
    if (!paths.length) {
      setError("Harbor needs a full file path to attach. Use Attach folder in the desktop app.");
      return;
    }
    try {
      await call("thread_attach_files", { id: active.id, paths });
      setAttached((current) => ({ ...current, [active.id]: [...(current[active.id] ?? []), ...paths] }));
    } catch (reason) {
      setError(`Could not attach those files. ${String(reason)}`);
    }
  }

  const mention = active ? mentionQuery(drafts[active.id] ?? "") : null;

  useEffect(() => {
    if (mention === null || !workspaceId) {
      setFileMentions([]);
      return;
    }
    let disposed = false;
    void call("fs_list", { workspaceId, path: "" })
      .then((entries) => {
        if (disposed) return;
        const needle = mention.toLowerCase();
        setFileMentions(entries.filter((entry) => !entry.directory && entry.name.toLowerCase().includes(needle)));
      })
      .catch(() => {
        if (!disposed) setFileMentions([]);
      });
    return () => {
      disposed = true;
    };
  }, [mention, workspaceId]);

  const canCreate = !creating && !checking && !!engine && !!workspace;
  return <div className="harbor-chat">
    <AppRail open={railOpen}><div className="harbor-rail-section">
      <div className="harbor-rail-heading"><h2>Chats</h2><span className="harbor-chip">Local</span></div>
      <Button variant="primary" disabled={!!workspace && !canCreate} onClick={() => void newThread()}>{creating ? "Creating…" : "New thread"}</Button>
      <FolderRail workspaces={workspaces} otherCount={other.length} selectedId={workspaceId}
        onAddWorkspace={() => setAdding(true)} onSelect={(id) => { setDestination("mode"); setWorkspaceId(id); setActive(null); }}>
        {threads?.length ? <ThreadList threads={threads} activeId={active?.id ?? null}
          onSelect={(thread) => { setDestination("mode"); setActive(thread); }} onPin={(id, pinned) => void pin(id, pinned)}
          onRename={(id, title) => void renameThread(id, title)} onDelete={(id) => void deleteThread(id)} />
          : <p className="harbor-rail-hint">{workspaceId && !threads ? "Loading threads…" : "Your conversations will appear here."}</p>}
      </FolderRail>
    </div></AppRail>
    <div className="harbor-stage-panel harbor-chat-main">
      {error ? <div className="harbor-status-banner" role="alert"><span>{error}</span><Button variant="ghost" onClick={() => { void reload(); void checkEngines(); }}>Try again</Button></div> : null}
      {active ? <>
        <ThreadHeader thread={threads?.find((thread) => thread.id === active.id) ?? active} workspace={workspace}
          disabled={!canCreate} onNew={() => void newThread()} />
        <ChangesPanel workspaceId={active.workspaceId} refreshToken={acp.turn} />
        {acp.loading && !acp.lines.length ? <div className="harbor-conversation-placeholder" role="status">Loading conversation…</div>
          : acp.lines.length ? <Transcript key={active.id} lines={acp.lines} />
          : <div className="harbor-conversation-placeholder"><span className="harbor-eyebrow">NEW CONVERSATION</span><h2>What are we working on?</h2><p>Ask a question about this project, describe a change, or work through an idea.</p>
            <div className="harbor-prompt-suggestions">{["Explain this project’s structure", "Help me plan my next change", "Review this project for improvements"].map((prompt) => <Button key={prompt} variant="ghost" onClick={() => setDrafts((current) => ({ ...current, [active.id]: prompt }))}>{prompt} <span aria-hidden="true">↗</span></Button>)}</div>
          </div>}
        {acp.permissions.map((request) => (
          <PermissionCard
            key={request.id}
            request={request}
            onResolve={(optionId, cancelled) => void acp.resolvePermission(request.id, optionId, cancelled)}
          />
        ))}
        {acp.sending ? <div className="harbor-chat-progress" role="status"><span className="harbor-live-dot" data-on="true" /> Waiting for {active.engineId}… <Button variant="ghost" onClick={() => void acp.cancel()}>Stop</Button></div> : null}
        {acp.error ? <div className="harbor-status-banner" role="alert"><span>{acp.error}</span><Button variant="ghost" onClick={acp.reload}>Reload conversation</Button></div> : null}
        {mention !== null ? (
          <MentionList
            label="Files"
            items={fileMentions.map((entry) => ({ id: entry.path, label: entry.name }))}
            onPick={(path) => {
              const id = active.id;
              setDrafts((current) => ({
                ...current,
                [id]: (current[id] ?? "").replace(/(?:^|\s)@[^\s]*$/, (chunk) => `${chunk.startsWith(" ") ? " " : ""}${path} `),
              }));
            }}
          />
        ) : null}
        <Composer value={drafts[active.id] ?? ""} onValueChange={(value) => setDrafts((current) => ({ ...current, [active.id]: value }))}
          disabled={acp.loading && !acp.sending} onSend={(value) => {
            const id = active.id;
            // The message is already in the transcript; holding it in the box
            // for the length of the turn only looks like it failed to send.
            setDrafts((current) => ({ ...current, [id]: "" }));
            void acp.send(value).then((success) => {
              if (!success) setDrafts((current) => current[id] ? current : { ...current, [id]: value });
            });
          }} textareaProps={{
            onKeyDown: (event) => {
              if (event.key === "Escape" && acp.sending) {
                event.preventDefault();
                void acp.cancel();
              }
            },
          }} controls={<>
            <EngineControls
              engines={ready}
              installable={installable}
              installing={installing}
              onInstall={(id) => void installAdapter(id)}
              engineId={active.engineId}
              options={acp.configOptions}
              choices={configChoice[active.id] ?? {}}
              busy={optionsBusy}
              disabled={acp.sending}
              onOpen={() => {
                if (acp.configOptions.length || optionsBusy) return;
                setOptionsBusy(true);
                void acp.loadConfigOptions().finally(() => setOptionsBusy(false));
              }}
              onEngineChange={(id) => void setThreadEngine(active.id, id)}
              onOptionChange={(optionId, value) => {
                setConfigChoice((current) => ({ ...current, [active.id]: { ...current[active.id], [optionId]: value } }));
                void call("thread_set_config", { id: active.id, optionId, value }).catch((reason) => {
                  setError(`Could not update engine options. ${String(reason)}`);
                });
              }}
            />
            <span className="harbor-composer-spacer" />
            <Button variant="ghost" disabled={acp.sending} onClick={() => void attachFolder()}>Attach folder</Button>
            <label className="harbor-chip">
              Attach files
              <input type="file" multiple hidden aria-label="Attach files" disabled={acp.sending} onChange={(event) => {
                void attachFiles(event.currentTarget.files);
                event.currentTarget.value = "";
              }} />
            </label>
            <ContextMeter lines={acp.lines} attached={attached[active.id]?.length ?? 0} />
            {(drafts[active.id] ?? "") ? null : <span className="harbor-composer-hint">Enter to send</span>}
          </>} />
      </> : <div className="harbor-start-page harbor-chat-start">
        <div className="harbor-start-mark"><Logo size={36} /></div>
        <span className="harbor-eyebrow">CHAT · IN YOUR FOLDERS</span>
        <h1>{workspace ? `Let’s work on ${workspace.title ?? "your project"}.` : "A conversation with your code."}</h1>
        <p>{workspace ? "Start a thread with an installed engine, or pick up a conversation in the sidebar." : "Choose a project folder to ask questions, plan changes, and keep the whole conversation in context."}</p>
        {loading ? <p role="status">Loading your workspace…</p> : !workspace ? <Button variant="primary" onClick={() => setAdding(true)}>Open a project folder <span aria-hidden="true">↗</span></Button> : <div className="harbor-chat-setup">
          <label>Chat engine<select aria-label="Chat engine" value={engine?.id ?? ""} disabled={checking || !ready.length} onChange={(event) => setEngineId(event.target.value)}>
            {ready.map((item) => <option key={item.id} value={item.id}>{item.displayName}</option>)}
            {!ready.length ? <option value="">{checking ? "Checking engines…" : "No chat engine ready"}</option> : null}
          </select></label>
          {ready.length ? <Button variant="primary" disabled={!canCreate} onClick={() => void newThread()}>{creating ? "Creating…" : "Start a thread"}</Button>
            : <><p>Install and sign in to an ACP-compatible engine such as OpenCode, then check again.</p><Button disabled={checking} onClick={() => void checkEngines()}>Check engines</Button></>}
        </div>}
        <small>{workspace?.folder ?? "Your files stay on your computer. No Harbor account required."}</small>
      </div>}
    </div>
    {adding ? <AddWorkspace onClose={() => setAdding(false)} onAdded={(added) => {
      setWorkspaces((current) => [added, ...current.filter((item) => item.id !== added.id)]);
      setWorkspaceId(added.id); setActive(null); setAdding(false); setDestination("mode");
    }} /> : null}
  </div>;
}
