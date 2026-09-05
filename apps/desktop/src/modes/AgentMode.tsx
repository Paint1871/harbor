import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AgentRecord } from "@harbor/schema/commands";
import { AgentRail } from "./agent/AgentRail";
import { AgentPage } from "./agent/AgentPage";
import { NewAgent } from "./agent/NewAgent";

export function AgentMode() {
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

  const agent = agents.find((item) => item.id === selected) ?? null;

  return (
    <div className="harbor-agent">
      <AgentRail agents={agents} selectedId={selected} onSelect={setSelected} onNew={() => setCreating(true)} />
      {agent ? <AgentPage agent={agent} /> : <div className="harbor-mode-body" />}
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
