import { useEffect, useState } from "react";
import { call } from "../../ipc";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine } from "@harbor/schema/commands";

interface LaunchWizardProps {
  onClose: () => void;
  onLaunch: (engineId: string) => void;
}

export function LaunchWizard({ onClose, onLaunch }: LaunchWizardProps) {
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [seat, setSeat] = useState("opencode");

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    void call("engines_detect")
      .then(setEngines)
      .catch(() => setEngines([]));
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const ready = engines.filter((engine) => engine.status === "ready");
  const selected = ready.find((engine) => engine.id === seat) ?? ready[0];

  return (
    <div className="harbor-modal-layer" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <div className="harbor-dialog harbor-launch-dialog" role="dialog" aria-modal="true" aria-labelledby="launch-title" onMouseDown={(event) => event.stopPropagation()}>
        <span className="harbor-eyebrow">WORKSPACE</span>
        <h2 id="launch-title">Launch a terminal</h2>
        <p>Choose the engine for this workspace. Harbor keeps the checkout shared and local.</p>
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
        <div className="harbor-dialog-actions">
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            onClick={() => onLaunch(selected?.id ?? "shell")}
            onKeyDown={(event) => {
              if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                onLaunch(selected?.id ?? "shell");
              }
            }}
          >
            Launch terminal
          </Button>
        </div>
      </div>
    </div>
  );
}
