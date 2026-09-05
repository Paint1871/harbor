import type { AgentRecord } from "@harbor/schema/commands";

interface GearPanelProps {
  agent: AgentRecord;
}

export function GearPanel({ agent }: GearPanelProps) {
  return (
    <aside className="harbor-gear" aria-label="Agent gear">
      <h3>Chats</h3>
      <p className="harbor-muted">Tabs for this teammate.</p>
      <h3>Memory</h3>
      <p className="harbor-muted">Facts only — never secrets.</p>
      <h3>Skills</h3>
      <p className="harbor-muted">Post-0.1.0.</p>
      <h3>Settings</h3>
      <p className="harbor-muted">Engine: {agent.engineId}</p>
    </aside>
  );
}
