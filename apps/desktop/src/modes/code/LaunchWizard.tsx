import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";
import type { DetectedEngine } from "@harbor/schema/commands";

interface LaunchWizardProps {
  onClose: () => void;
  onLaunch: (engineId: string) => void;
}

export function LaunchWizard({ onClose, onLaunch }: LaunchWizardProps) {
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [seat, setSeat] = useState("opencode");

  useEffect(() => {
    void invoke<DetectedEngine[]>("engines_detect")
      .then(setEngines)
      .catch(() => setEngines([]));
  }, []);

  const ready = engines.filter((engine) => engine.status === "ready");
  const selected = ready.find((engine) => engine.id === seat) ?? ready[0];

  return (
    <div className="harbor-dialog" role="dialog" aria-labelledby="launch-title">
      <Card>
        <h2 id="launch-title">Launch wizard</h2>
        <p>Preset: Solo · Isolation: Shared checkout</p>
        <label>
          Seat 1
          <select value={selected?.id ?? ""} onChange={(event) => setSeat(event.target.value)}>
            {ready.map((engine) => (
              <option key={engine.id} value={engine.id}>
                {engine.displayName} ({engine.path || engine.status})
              </option>
            ))}
            {ready.length === 0 ? <option value="shell">Terminal</option> : null}
          </select>
        </label>
        <div className="harbor-welcome-actions">
          <Button
            variant="primary"
            onClick={() => onLaunch(selected?.id ?? "shell")}
            onKeyDown={(event) => {
              if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                onLaunch(selected?.id ?? "shell");
              }
            }}
          >
            Launch 1 terminal
          </Button>
          <Button onClick={onClose}>Cancel</Button>
        </div>
      </Card>
    </div>
  );
}
