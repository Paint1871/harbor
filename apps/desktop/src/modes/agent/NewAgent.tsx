import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine } from "@harbor/schema/commands";
import { FacePicker } from "./FacePicker";

interface NewAgentProps {
  onCreate: (input: { name: string; brief: string; engineId: string; faceIndex: number }) => void;
  onClose: () => void;
}

export function NewAgent({ onCreate, onClose }: NewAgentProps) {
  const [name, setName] = useState("");
  const [brief, setBrief] = useState("");
  const [engineId, setEngineId] = useState("opencode");
  const [faceIndex, setFaceIndex] = useState(0);
  const [engines, setEngines] = useState<DetectedEngine[]>([]);

  useEffect(() => {
    void invoke<DetectedEngine[]>("engines_detect")
      .then(setEngines)
      .catch(() => setEngines([]));
  }, []);

  const ready = engines.filter((engine) => engine.status === "ready" && engine.supportsChat);
  const usable = ready.length > 0 ? ready : engines.filter((engine) => engine.status === "ready");
  const disabled = name.trim().length === 0 || usable.length === 0;

  return (
    <div className="harbor-dialog" role="dialog" aria-labelledby="new-agent-title">
      <h2 id="new-agent-title">New Agent</h2>
      <label>
        Name
        <input value={name} maxLength={40} onChange={(event) => setName(event.target.value)} />
      </label>
      <label>
        Brief
        <textarea value={brief} onChange={(event) => setBrief(event.target.value)} />
      </label>
      <label>
        Engine
        <select value={engineId} onChange={(event) => setEngineId(event.target.value)}>
          {usable.map((engine) => (
            <option key={engine.id} value={engine.id}>
              {engine.displayName}
            </option>
          ))}
          {usable.length === 0 ? <option value="">Install an engine to create</option> : null}
        </select>
      </label>
      <FacePicker value={faceIndex} onChange={setFaceIndex} name={name || "Agent"} />
      <div className="harbor-welcome-actions">
        <Button variant="primary" disabled={disabled} onClick={() => onCreate({ name: name.trim(), brief, engineId, faceIndex })}>
          Create
        </Button>
        <Button onClick={onClose}>Cancel</Button>
      </div>
    </div>
  );
}
