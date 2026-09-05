import { useState } from "react";
import { Composer } from "@harbor/ui/Composer";
import { Button } from "@harbor/ui/Button";
import type { AgentRecord } from "@harbor/schema/commands";
import { Face } from "./Face";
import { GearPanel } from "./GearPanel";
import { Transcript, type TranscriptLine } from "../../chrome/Transcript";

interface AgentPageProps {
  agent: AgentRecord;
}

export function AgentPage({ agent }: AgentPageProps) {
  const [draft, setDraft] = useState("");
  const [gear, setGear] = useState(false);
  const [tab, setTab] = useState<"chats" | "skills" | "settings">("chats");
  const [chats, setChats] = useState([{ id: "chat-1", title: "New chat" }]);
  const [activeChat, setActiveChat] = useState("chat-1");
  const [lines, setLines] = useState<TranscriptLine[]>([]);

  const engineLabel = agent.engineId.replace(/-/g, " ");

  return (
    <div className="harbor-agent-page">
      <header className="harbor-agent-header">
        <Face name={agent.name} index={agent.faceIndex} />
        <div className="harbor-agent-heading">
          <h2>{agent.name}</h2>
          <p>Powered by {engineLabel}</p>
        </div>
        <div className="harbor-agent-header-actions">
          <div className="harbor-agent-tabs" role="tablist" aria-label="Agent sections">
            <button type="button" role="tab" aria-selected={tab === "chats"} onClick={() => setTab("chats")}>
              Chats
            </button>
            <button type="button" role="tab" aria-selected={tab === "skills"} onClick={() => setTab("skills")}>
              Skills
            </button>
            <button type="button" role="tab" aria-selected={tab === "settings"} onClick={() => setGear((open) => !open)}>
              Settings
            </button>
          </div>
          <Button
            variant="primary"
            onClick={() => {
              const id = `chat-${chats.length + 1}`;
              setChats((current) => [...current, { id, title: "New chat" }]);
              setActiveChat(id);
              setTab("chats");
            }}
          >
            New chat
          </Button>
        </div>
      </header>
      {gear ? <GearPanel agent={agent} /> : null}
      <div className="harbor-agent-body">
        <aside className="harbor-agent-chats" aria-label="Chats">
          {chats.map((chat) => (
            <button
              key={chat.id}
              type="button"
              className="harbor-chat-tab"
              data-selected={activeChat === chat.id}
              onClick={() => setActiveChat(chat.id)}
            >
              {chat.title}
            </button>
          ))}
        </aside>
        <div className="harbor-agent-main">
          <Transcript lines={lines} />
          <Composer
            value={draft}
            onValueChange={setDraft}
            onSend={(value) => {
              setLines((current) => [
                ...current,
                { id: `${Date.now()}`, text: value, role: "user" },
              ]);
              setDraft("");
            }}
            controls={
              <>
                <span className="harbor-chip">{engineLabel}</span>
                <span className="harbor-chip">High</span>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
