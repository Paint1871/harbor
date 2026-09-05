import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import { Pill } from "@harbor/ui/Pill";
import type { AgentRecord } from "@harbor/schema/commands";
import { Face } from "./Face";

interface AgentRailProps {
  agents: AgentRecord[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
}

export function AgentRail({ agents, selectedId, onSelect, onNew }: AgentRailProps) {
  const pinned = agents.filter((agent) => agent.pinned);
  const rest = agents.filter((agent) => !agent.pinned);
  return (
    <div className="harbor-agent-rail">
      <Button variant="primary" onClick={onNew}>
        New Agent
      </Button>
      {pinned.length > 0 ? (
        <div className="harbor-pin-band">
          {pinned.map((agent) => (
            <RailRow
              key={agent.id}
              label={agent.name}
              description={agent.brief}
              leading={<Face name={agent.name} index={agent.faceIndex} />}
              selected={selectedId === agent.id}
              onClick={() => onSelect(agent.id)}
            />
          ))}
        </div>
      ) : null}
      {rest.map((agent) => (
        <RailRow
          key={agent.id}
          label={agent.name}
          description={agent.brief}
          leading={<Face name={agent.name} index={agent.faceIndex} />}
          trailing={<Pill>idle</Pill>}
          selected={selectedId === agent.id}
          onClick={() => onSelect(agent.id)}
        />
      ))}
    </div>
  );
}
