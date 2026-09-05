import { useState } from "react";
import { Composer } from "@harbor/ui/Composer";
import { Button } from "@harbor/ui/Button";
import type { AgentRecord } from "@harbor/schema/commands";
import { Face } from "./Face";
import { GearPanel } from "./GearPanel";

interface AgentPageProps {
  agent: AgentRecord;
}

export function AgentPage({ agent }: AgentPageProps) {
  const [draft, setDraft] = useState("");
  const [gear, setGear] = useState(false);
  const [lines, setLines] = useState<string[]>([]);

  return (
    <div className="harbor-agent-page">
      <header>
        <Face name={agent.name} index={agent.faceIndex} />
        <div>
          <h2>{agent.name}</h2>
          <p>{agent.brief || "No brief yet."}</p>
        </div>
        <Button onClick={() => setGear((open) => !open)} aria-label="Gear">
          Gear
        </Button>
      </header>
      {gear ? <GearPanel agent={agent} /> : null}
      <div className="harbor-chat-transcript">
        {lines.map((line, index) => (
          <p key={index}>{line}</p>
        ))}
      </div>
      <Composer
        value={draft}
        onValueChange={setDraft}
        onSend={(value) => {
          setLines((current) => [...current, value]);
          setDraft("");
        }}
      />
    </div>
  );
}
