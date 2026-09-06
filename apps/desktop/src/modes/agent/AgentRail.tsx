import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import type { AgentRecord } from "@harbor/schema/commands";
import { Face } from "./Face";
import { useChrome } from "../../chrome/chrome-context";

interface AgentRailProps {
  agents: AgentRecord[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
}

export function AgentRail({ agents, selectedId, onSelect, onNew }: AgentRailProps) {
  const { setDestination } = useChrome();
  const pinned = agents.filter((agent) => agent.pinned);
  const rest = agents.filter((agent) => !agent.pinned);
  return (
    <div className="harbor-rail-section">
      <div className="harbor-rail-heading">
        <h2>Agents</h2>
        <Button
          size="icon"
          variant="ghost"
          aria-label="Add to Agents"
          title="Add to Agents"
          onClick={() => { setDestination("mode"); onNew(); }}
        >
          <span aria-hidden="true">+</span>
          <span className="harbor-sr-only">New Agent</span>
        </Button>
      </div>
      <div className="harbor-roster" aria-label="Agent roster">
        {pinned.length > 0 ? (
          <div className="harbor-pin-band">
            {pinned.map((agent) => (
              <RailRow
                key={agent.id}
                label={agent.name}
                description={agent.brief}
                leading={<Face name={agent.name} index={agent.faceIndex} />}
                selected={selectedId === agent.id}
                onClick={() => {
                  setDestination("mode");
                  onSelect(agent.id);
                }}
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
            selected={selectedId === agent.id}
            onClick={() => {
              setDestination("mode");
              onSelect(agent.id);
            }}
          />
        ))}
      </div>
    </div>
  );
}
