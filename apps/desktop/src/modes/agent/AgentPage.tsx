import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Composer } from "@harbor/ui/Composer";
import { Button } from "@harbor/ui/Button";
import type { AgentRecord } from "@harbor/schema/commands";
import { Face } from "./Face";
import { GearPanel } from "./GearPanel";
import { useAgentChat } from "./useAgentChat";
import { EnginePicker } from "../chat/EnginePicker";
import { ContextMeter } from "../chat/ContextMeter";
import { PermissionCard } from "../chat/PermissionCard";
import { MentionList, mentionQuery } from "../../chrome/MentionList";
import { Transcript } from "../../chrome/Transcript";
import { useOptionalChrome } from "../../chrome/chrome-context";
import { settingsGet, settingsSet } from "../../settings";

interface AgentPageProps {
  agent: AgentRecord;
  onAgentChange?: (agent: AgentRecord) => void;
  onOpenSkills?: () => void;
  onOpenPlugins?: () => void;
}

export function AgentPage({ agent, onAgentChange, onOpenSkills, onOpenPlugins }: AgentPageProps) {
  const chrome = useOptionalChrome();
  const [draft, setDraft] = useState("");
  const [tab, setTab] = useState<"chats" | "skills" | "settings">("chats");
  const [configChoice, setConfigChoice] = useState<Record<string, Record<string, string>>>({});
  const [teammates, setTeammates] = useState<AgentRecord[]>([]);
  const [mailError, setMailError] = useState<string | null>(null);
  const chat = useAgentChat(agent.id);
  const mention = mentionQuery(draft);
  const mentionItems = teammates
    .filter((item) => item.id !== agent.id)
    .filter((item) => !mention || item.name.toLowerCase().includes(mention.toLowerCase()))
    .map((item) => ({ id: item.id, label: item.name }));

  useEffect(() => {
    const watched = chrome?.mode === "agent" && chrome?.destination === "mode";
    const sessionRef = watched ? chat.activeId : null;
    void invoke("session_watch", { sessionRef }).catch(() => undefined);
    return () => { void invoke("session_watch", { sessionRef: null }).catch(() => undefined); };
  }, [chat.activeId, chrome?.destination, chrome?.mode]);

  useEffect(() => {
    void invoke<AgentRecord[]>("agent_list")
      .then(setTeammates)
      .catch(() => setTeammates([]));
  }, [agent.id]);
  useEffect(() => {
    let active = true;
    void settingsGet("pending_agent_prompt").then((value) => {
      if (!active || typeof value !== "string" || !value.trim()) return;
      setDraft(value);
      void settingsSet("pending_agent_prompt", null);
    });
    return () => { active = false; };
  }, [agent.id, chrome?.destination]);
  useEffect(() => {
    if (chrome?.destination !== "mode") return;
    chat.reload();
    // The routine and skill destinations can create or prepare work while the
    // Agent view remains mounted behind them; refresh when the view returns.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [agent.id, chrome?.destination]);
  const [optionsBusy, setOptionsBusy] = useState(false);
  const engineLabel = agent.engineId.replace(/-/g, " ");
  const selectedOptions = (chat.activeId ? configChoice[chat.activeId] : undefined) ?? {};
  const workingCount = Math.max(
    chat.chats.filter((item) => item.status === "running").length,
    chat.sending ? 1 : 0,
    agent.trailing?.kind === "running" ? agent.trailing.n : 0,
  );
  const needsYou = chat.permissions.length > 0 || agent.trailing?.kind === "needs_you" || chat.chats.some((item) => item.status === "needs_you");

  return (
    <div className="harbor-agent-page">
      <header className="harbor-agent-header">
        <Face name={agent.name} index={agent.faceIndex} />
        <div className="harbor-agent-heading">
          <h2>{agent.name}</h2>
          <p>Powered by {engineLabel}</p>
        </div>
        <div className="harbor-agent-header-actions">
          <ContextMeter lines={chat.lines} />
          <span className="harbor-agent-status">
            <span className="harbor-live-dot" data-on={needsYou || workingCount > 0} />
            {needsYou ? "Needs you" : `${workingCount} working`}
          </span>
          <div className="harbor-agent-tabs" role="tablist" aria-label="Agent sections">
            <button type="button" role="tab" aria-selected={tab === "chats"} onClick={() => setTab("chats")}>
              Chats
            </button>
            <button type="button" role="tab" aria-selected={tab === "skills"} onClick={() => setTab("skills")}>
              Skills
            </button>
            <button type="button" role="tab" aria-selected={tab === "settings"} onClick={() => setTab("settings")}>
              Settings
            </button>
          </div>
          <Button variant="primary" disabled={chat.creating} onClick={() => { setTab("chats"); void chat.createChat(); }}>
            {chat.creating ? "Creating…" : <>New chat <span className="harbor-button-chevron" aria-hidden="true">⌄</span></>}
          </Button>
        </div>
      </header>
      <div className="harbor-agent-body">
        {tab === "settings" ? (
          <GearPanel
            agent={agent}
            chats={chat.chats}
            onAgentChange={onAgentChange}
            onOpenChat={(chatId) => {
              chat.setActiveId(chatId);
              setTab("chats");
            }}
            onOpenSkills={onOpenSkills}
            onOpenPlugins={onOpenPlugins}
          />
        ) : (
          <>
            <aside className="harbor-agent-chats" aria-label="Chats">
              <div className="harbor-agent-chats-heading">
                <span className="harbor-eyebrow">CHATS</span>
                <span className="harbor-chip">{chat.chats.length}</span>
              </div>
              {chat.chats.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className="harbor-chat-tab"
                  data-selected={chat.activeId === item.id}
                  onClick={() => { chat.setActiveId(item.id); setTab("chats"); }}
                >
                  <span className="harbor-chat-tab-title">{item.title}</span>
                  {item.status === "running" ? <span className="harbor-live-dot" data-on="true" title="Working" /> : null}
                  {item.status === "needs_you" ? <span className="harbor-chat-tab-flag">Needs you</span> : null}
                </button>
              ))}
              {!chat.chats.length ? <p className="harbor-muted">{chat.loading ? "Loading chats…" : "No chats yet."}</p> : null}
            </aside>
            <div className="harbor-agent-main">
              {tab === "skills" ? (
                <div className="harbor-agent-info-panel">
                  <div className="harbor-info-mark" aria-hidden="true">✦</div>
                  <span className="harbor-eyebrow">SKILLS</span>
                  <h2>Give this teammate a playbook.</h2>
                  <p>Start with a reusable local brief, then tune the teammate’s memory, places, and engine in Settings.</p>
                  <div className="harbor-agent-info-actions">
                    <Button variant="primary" onClick={() => onOpenSkills?.()}>Browse skills <span aria-hidden="true">↗</span></Button>
                    <Button variant="ghost" onClick={() => setTab("settings")}>Open agent settings</Button>
                  </div>
                </div>
              ) : (
                <>
              {chat.historyLoading && !chat.lines.length ? (
                <div className="harbor-conversation-placeholder" role="status">Loading conversation…</div>
              ) : chat.lines.length ? (
                <Transcript key={chat.activeId ?? "empty"} lines={chat.lines} />
              ) : (
                <div className="harbor-conversation-placeholder">
                  <span className="harbor-eyebrow">NEW CONVERSATION</span>
                  <h2>What should we take on?</h2>
                  <p>Ask this teammate for a plan, a review, or a first draft. Each chat stays with this agent.</p>
                  <div className="harbor-prompt-suggestions">
                    {["Plan a task", "Review a change", "Draft a handoff"].map((prompt) => (
                      <Button key={prompt} variant="ghost" onClick={() => setDraft(prompt)}>
                        {prompt}<span aria-hidden="true">↗</span>
                      </Button>
                    ))}
                  </div>
                </div>
              )}
              {chat.permissions.map((request) => (
                <PermissionCard
                  key={request.id}
                  request={request}
                  onResolve={(optionId, cancelled) => void chat.resolvePermission(request.id, optionId, cancelled)}
                />
              ))}
              {chat.sending ? (
                <div className="harbor-chat-progress" role="status">
                  <span className="harbor-live-dot" data-on="true" /> Waiting for {engineLabel}…
                  <Button variant="ghost" onClick={() => void chat.cancel()}>Stop</Button>
                </div>
              ) : null}
              {chat.error ? (
                <div className="harbor-status-banner" role="alert">
                  <span>{chat.error}</span>
                  <Button variant="ghost" onClick={chat.reload}>Try again</Button>
                </div>
              ) : null}
              {mailError ? (
                <div className="harbor-status-banner" role="alert">
                  <span>{mailError}</span>
                </div>
              ) : null}
              {mention !== null ? (
                <MentionList
                  label="Teammates"
                  items={mentionItems}
                  onPick={(id) => {
                    const to = teammates.find((item) => item.id === id);
                    if (!to) return;
                    const body = draft.replace(/(?:^|\s)@[^\s]*$/, "").trim() || `Handoff from ${agent.name}`;
                    setMailError(null);
                    void invoke("mail_send", { fromAgentId: agent.id, toAgentId: to.id, body })
                      .then(() => setDraft(""))
                      .catch((reason) => setMailError(`Mail could not be sent. ${String(reason)}`));
                  }}
                />
              ) : null}
              <Composer
                value={draft}
                onValueChange={setDraft}
                disabled={chat.historyLoading && !chat.sending}
                onSend={(value) => {
                  const named = teammates.find((item) => item.id !== agent.id && value.toLowerCase().startsWith(`@${item.name.toLowerCase()}`));
                  if (named) {
                    const body = value.slice(named.name.length + 1).trim() || `Handoff from ${agent.name}`;
                    setMailError(null);
                    void invoke("mail_send", { fromAgentId: agent.id, toAgentId: named.id, body })
                      .then(() => setDraft(""))
                      .catch((reason) => setMailError(`Mail could not be sent. ${String(reason)}`));
                    return;
                  }
                  setDraft("");
                  void chat.send(value).then((success) => {
                    if (!success) setDraft((current) => current || value);
                  });
                }}
                textareaProps={{
                  onKeyDown: (event) => {
                    if (event.key === "Escape" && chat.sending) {
                      event.preventDefault();
                      void chat.cancel();
                    }
                  },
                }}
                controls={
                  <>
                    <EnginePicker
                      engines={[]}
                      engineId={engineLabel}
                      options={chat.configOptions}
                      choices={selectedOptions}
                      busy={optionsBusy}
                      onOpen={() => {
                        if (chat.configOptions.length) return;
                        setOptionsBusy(true);
                        void chat.loadConfigOptions().finally(() => setOptionsBusy(false));
                      }}
                      onOptionChange={(optionId: string, value: string) => {
                        const chatId = chat.activeId;
                        if (chatId) setConfigChoice((current) => ({ ...current, [chatId]: { ...current[chatId], [optionId]: value } }));
                        void chat.setConfig(optionId, value);
                      }}
                    />
                    <span className="harbor-muted">Enter to send · Shift + Enter for a new line</span>
                  </>
                }
              />
                </>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
