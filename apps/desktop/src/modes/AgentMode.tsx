import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Logo } from "@harbor/ui/Logo";
import type { AgentRecord } from "@harbor/schema/commands";
import { AppRail } from "../chrome/AppRail";
import { AgentRail } from "./agent/AgentRail";
import { AgentPage } from "./agent/AgentPage";
import { NewAgent } from "./agent/NewAgent";
import { useChrome } from "../chrome/chrome-context";

export function AgentMode({ railOpen = true }: { railOpen?: boolean }) {
  const { setDestination } = useChrome();
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setAgents(await invoke<AgentRecord[]>("agent_list"));
    } catch {
      setError("Your agents could not be loaded. Try again in the Harbor desktop app.");
    } finally {
      setLoading(false);
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
          <AgentPage
            agent={agent}
            onAgentChange={(updated) => {
              setAgents((current) => current.map((item) => (item.id === updated.id ? { ...item, ...updated } : item)));
            }}
            onOpenSkills={() => setDestination("skills")}
            onOpenPlugins={() => setDestination("plugins")}
          />
        ) : (
          <div className="harbor-start-page">
            <div className="harbor-start-mark"><Logo size={36} /></div>
            <span className="harbor-eyebrow">YOUR LOCAL WORKSPACE</span>
            <h1>A teammate for your next idea.</h1>
            <p>Give an agent a name and a brief. Keep its conversations and context together, ready for whatever comes next.</p>
            {loading ? <p role="status">Loading your agents…</p> : error ? (
              <div className="harbor-inline-error" role="alert">
                <p>{error}</p>
                <Button onClick={() => void reload()}>Try again</Button>
              </div>
            ) : (
              <Button variant="primary" onClick={() => setCreating(true)}>Create your first agent <span aria-hidden="true">↗</span></Button>
            )}
            <div className="harbor-start-steps">
              <div><span>01</span><h3>Make it yours</h3><p>A name, a face, and a clear brief.</p></div>
              <div><span>02</span><h3>Choose an engine</h3><p>Connect an installed coding agent.</p></div>
              <div><span>03</span><h3>Start a conversation</h3><p>Describe the work. Review the result.</p></div>
            </div>
            <small>Local first · Your engines · No account required</small>
          </div>
        )}
      </div>
      {creating ? (
        <NewAgent
          onClose={() => setCreating(false)}
          onCreate={async (input) => {
            const created = await invoke<AgentRecord>("agent_create", { input });
            setAgents((current) => [...current, created]);
            setSelected(created.id);
            setCreating(false);
          }}
        />
      ) : null}
    </div>
  );
}
