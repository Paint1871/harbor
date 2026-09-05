import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Composer } from "@harbor/ui/Composer";
import type { AgentRecord } from "@harbor/schema/commands";
import { AppRail } from "../chrome/AppRail";
import { AgentRail } from "./agent/AgentRail";
import { AgentPage } from "./agent/AgentPage";
import { NewAgent } from "./agent/NewAgent";

export function AgentMode({ railOpen = true }: { railOpen?: boolean }) {
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const reload = useCallback(async () => {
    try {
      setAgents(await invoke<AgentRecord[]>("agent_list"));
    } catch {
      setAgents([]);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const agent = agents.find((item) => item.id === selected) ?? agents[0] ?? null;

  return (
    <div className="harbor-agent">
      {railOpen ? (
        <AppRail>
          <AgentRail
            agents={agents}
            selectedId={agent?.id ?? null}
            onSelect={setSelected}
            onNew={() => setCreating(true)}
          />
        </AppRail>
      ) : null}
      <div className="harbor-stage-panel">
        {agent ? (
          <AgentPage agent={agent} />
        ) : (
          <div className="harbor-agent-page">
            <p className="harbor-muted" style={{ padding: 16 }}>
              Create an agent to start a teammate chat.
            </p>
            <div className="harbor-agent-main">
              <div className="harbor-chat-transcript" />
              <Composer value="" onValueChange={() => undefined} onSend={() => undefined} disabled />
            </div>
          </div>
        )}
      </div>
      {creating ? (
        <NewAgent
          onClose={() => setCreating(false)}
          onCreate={(input) => {
            void invoke<AgentRecord>("agent_create", { input })
              .then((created) => {
                setSelected(created.id);
                setCreating(false);
                return reload();
              })
              .catch(() => setCreating(false));
          }}
        />
      ) : null}
    </div>
  );
}
