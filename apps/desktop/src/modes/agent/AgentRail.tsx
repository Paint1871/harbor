import type { DragEvent } from "react";
import { Button } from "@harbor/ui/Button";
import { RailRow } from "@harbor/ui/RailRow";
import type { AgentRecord, AgentTrailing } from "@harbor/schema/commands";
import { Face } from "./Face";
import { useChrome } from "../../chrome/chrome-context";

interface AgentRailProps {
  agents: AgentRecord[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  onPin?: (id: string, pinned: boolean) => void;
}

const AGENT_DRAG = "text/harbor-agent";

function recency(at: number): string {
  const delta = Date.now() / 1000 - at;
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h`;
  return `${Math.floor(delta / 86400)}d`;
}

function trailingLabel(trailing: AgentTrailing | undefined): string {
  if (!trailing || trailing.kind === "idle") return "";
  if (trailing.kind === "running") return trailing.n > 1 ? String(trailing.n) : "live";
  if (trailing.kind === "needs_you") return "Needs you";
  return recency(trailing.at);
}

export function AgentRail({ agents, selectedId, onSelect, onNew, onPin }: AgentRailProps) {
  const { setDestination } = useChrome();
  const pinned = agents.filter((agent) => agent.pinned);
  const rest = agents.filter((agent) => !agent.pinned);

  function dropPin(event: DragEvent, pinned: boolean) {
    event.preventDefault();
    const id = event.dataTransfer.getData(AGENT_DRAG) || event.dataTransfer.getData("text/plain");
    if (!id) return;
    onPin?.(id, pinned);
  }

  function row(agent: AgentRecord) {
    const description = agent.lastLine?.trim() || agent.brief;
    const trailing = trailingLabel(agent.trailing);
    return (
      <div
        key={agent.id}
        className="harbor-roster-item"
        draggable={!!onPin}
        onDragStart={(event) => {
          event.dataTransfer.setData(AGENT_DRAG, agent.id);
          event.dataTransfer.setData("text/plain", agent.id);
          event.dataTransfer.effectAllowed = "move";
        }}
      >
        <RailRow
          label={agent.name}
          description={description}
          leading={<Face name={agent.name} index={agent.faceIndex} />}
          trailing={trailing || undefined}
          selected={selectedId === agent.id}
          onClick={() => {
            setDestination("mode");
            onSelect(agent.id);
          }}
        />
        {onPin ? (
          <button
            type="button"
            className="harbor-pin"
            data-on={agent.pinned}
            aria-label={agent.pinned ? `Unpin ${agent.name}` : `Pin ${agent.name}`}
            title={agent.pinned ? "Unpin" : "Pin"}
            onClick={() => onPin(agent.id, !agent.pinned)}
          >
            {agent.pinned ? "★" : "☆"}
          </button>
        ) : null}
      </div>
    );
  }

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
        {pinned.length > 0 || (onPin && agents.length > 0) ? (
          <div
            className="harbor-pin-band"
            data-empty={pinned.length === 0}
            onDragOver={(event) => {
              if (!onPin) return;
              event.preventDefault();
              event.dataTransfer.dropEffect = "move";
            }}
            onDrop={(event) => dropPin(event, true)}
          >
            {pinned.length ? pinned.map(row) : <p className="harbor-muted">Drag a teammate here to pin.</p>}
          </div>
        ) : null}
        <div
          onDragOver={(event) => {
            if (!onPin) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = "move";
          }}
          onDrop={(event) => dropPin(event, false)}
        >
          {rest.map(row)}
        </div>
      </div>
    </div>
  );
}
