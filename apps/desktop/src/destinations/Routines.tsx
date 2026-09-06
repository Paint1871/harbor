import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import type { AgentRecord } from "@harbor/schema/commands";
import { settingsGet, settingsSet } from "../settings";
import { useChrome } from "../chrome/chrome-context";

type Frequency = "weekdays" | "weekly" | "manual";

interface Routine {
  id: string;
  name: string;
  brief: string;
  frequency: Frequency;
  agentId: string;
  enabled: boolean;
}

const FREQUENCIES: { value: Frequency; label: string }[] = [
  { value: "weekdays", label: "Every weekday" },
  { value: "weekly", label: "Once a week" },
  { value: "manual", label: "Manual only" },
];

function readRoutines(value: unknown): Routine[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is Routine => {
    if (!item || typeof item !== "object") return false;
    const row = item as Partial<Routine>;
    return typeof row.id === "string"
      && typeof row.name === "string"
      && typeof row.brief === "string"
      && (row.frequency === "weekdays" || row.frequency === "weekly" || row.frequency === "manual")
      && typeof row.agentId === "string"
      && typeof row.enabled === "boolean";
  });
}

export function Routines() {
  const { onModeChange } = useChrome();
  const [routines, setRoutines] = useState<Routine[]>([]);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [brief, setBrief] = useState("");
  const [frequency, setFrequency] = useState<Frequency>("weekdays");
  const [agentId, setAgentId] = useState("");
  const [running, setRunning] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void settingsGet("routines_local").then((value) => {
      if (active) setRoutines(readRoutines(value));
    });
    void invoke<AgentRecord[]>("agent_list")
      .then((listed) => {
        if (!active) return;
        setAgents(listed);
        setAgentId((current) => current || listed[0]?.id || "");
      })
      .catch(() => undefined);
    return () => { active = false; };
  }, []);

  const persist = useCallback((next: Routine[]) => {
    setRoutines(next);
    void settingsSet("routines_local", next);
  }, []);

  const activeCount = routines.filter((routine) => routine.enabled).length;
  const agentName = useMemo(() => new Map(agents.map((agent) => [agent.id, agent.name])), [agents]);

  function resetForm() {
    setName("");
    setBrief("");
    setFrequency("weekdays");
    setAgentId((current) => current || agents[0]?.id || "");
  }

  function addRoutine() {
    const trimmedName = name.trim();
    const trimmedBrief = brief.trim();
    if (!trimmedName || !trimmedBrief || !agentId) return;
    const next: Routine = {
      id: crypto.randomUUID(),
      name: trimmedName,
      brief: trimmedBrief,
      frequency,
      agentId,
      enabled: true,
    };
    persist([next, ...routines]);
    setOpen(false);
    resetForm();
    setNotice("Routine saved locally.");
  }

  async function runRoutine(routine: Routine) {
    setRunning(routine.id);
    setNotice(null);
    try {
      const chat = await invoke<{ id: string }>("agent_chat_create", { agentId: routine.agentId });
      await settingsSet("pending_agent_prompt", routine.brief);
      setNotice(`Started a new chat with ${agentName.get(routine.agentId) ?? "your teammate"}.`);
      onModeChange("agent");
      void chat;
    } catch (reason) {
      setNotice(`Could not start this routine. ${String(reason)}`);
    } finally {
      setRunning(null);
    }
  }

  return (
    <section className="harbor-destination-page harbor-routines" aria-label="Routines">
      <header className="harbor-destination-heading">
        <div>
          <span className="harbor-eyebrow">AGENT SCHEDULES</span>
          <h1>Routines</h1>
          <p>Give a teammate a repeatable brief. Runs stay on this device while Harbor is open.</p>
        </div>
        <Button variant="primary" onClick={() => { setNotice(null); setOpen(true); }}>
          New routine <span aria-hidden="true">+</span>
        </Button>
      </header>

      <div className="harbor-destination-stats" aria-label="Routine summary">
        <span><strong>{activeCount}</strong> active</span>
        <span><strong>{routines.length}</strong> total</span>
        <span className="harbor-muted">Local schedule</span>
      </div>

      {open ? (
        <form className="harbor-routine-form" onSubmit={(event) => { event.preventDefault(); addRoutine(); }}>
          <div className="harbor-routine-form-heading">
            <div><span className="harbor-eyebrow">NEW ROUTINE</span><h2>Set the brief</h2></div>
            <Button variant="ghost" type="button" aria-label="Close routine form" onClick={() => { setOpen(false); resetForm(); }}>×</Button>
          </div>
          <label>Routine name<input autoFocus value={name} maxLength={80} placeholder="Morning signal scan" onChange={(event) => setName(event.target.value)} /></label>
          <label>What should the teammate do?<textarea value={brief} maxLength={600} rows={3} placeholder="Review the latest notes and prepare the three decisions I need to make." onChange={(event) => setBrief(event.target.value)} /></label>
          <div className="harbor-routine-form-grid">
            <label>Teammate<select value={agentId} onChange={(event) => setAgentId(event.target.value)} disabled={!agents.length}>
              {!agents.length ? <option value="">No agents available</option> : agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
            </select></label>
            <label>Schedule<select value={frequency} onChange={(event) => setFrequency(event.target.value as Frequency)}>{FREQUENCIES.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>
          </div>
          {!agents.length ? <p className="harbor-inline-error">Create an agent first, then come back to schedule its work.</p> : null}
          <div className="harbor-dialog-actions"><Button type="button" variant="ghost" onClick={() => { setOpen(false); resetForm(); }}>Cancel</Button><Button type="submit" variant="primary" disabled={!name.trim() || !brief.trim() || !agentId}>Save routine</Button></div>
        </form>
      ) : null}

      {routines.length ? (
        <div className="harbor-routine-list">
          {routines.map((routine) => (
            <article key={routine.id} className="harbor-routine-card" data-enabled={routine.enabled}>
              <div className="harbor-routine-card-main">
                <div className="harbor-routine-card-top"><span className="harbor-routine-icon" aria-hidden="true">↻</span><span className="harbor-chip">{FREQUENCIES.find((item) => item.value === routine.frequency)?.label}</span><span className="harbor-routine-agent">{agentName.get(routine.agentId) ?? "Missing agent"}</span></div>
                <h2>{routine.name}</h2>
                <p>{routine.brief}</p>
              </div>
              <div className="harbor-routine-card-actions">
                <label className="harbor-routine-toggle"><input type="checkbox" checked={routine.enabled} onChange={(event) => persist(routines.map((item) => item.id === routine.id ? { ...item, enabled: event.target.checked } : item))} /><span>{routine.enabled ? "On" : "Off"}</span></label>
                <Button variant="ghost" disabled={!routine.enabled || running === routine.id} onClick={() => void runRoutine(routine)}>{running === routine.id ? "Starting…" : "Run now"}</Button>
                <Button variant="ghost" onClick={() => persist(routines.filter((item) => item.id !== routine.id))}>Remove</Button>
              </div>
            </article>
          ))}
        </div>
      ) : (
        <div className="harbor-routines-empty">
          <div className="harbor-routines-empty-mark" aria-hidden="true">↻</div>
          <span className="harbor-eyebrow">NO ROUTINES YET</span>
          <h2>Make useful work repeatable.</h2>
          <p>Create a local schedule for one of your teammates. You can review the brief before it runs and pause it whenever you want.</p>
          <Button variant="secondary" onClick={() => { setNotice(null); setOpen(true); }}>Create your first routine <span aria-hidden="true">↗</span></Button>
        </div>
      )}
      {notice ? <p className="harbor-destination-notice" role="status">{notice}</p> : null}
    </section>
  );
}
