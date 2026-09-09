import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../../ipc";
import { Button } from "@harbor/ui/Button";
import type { DetectedEngine } from "@harbor/schema/commands";
import { FacePicker } from "./FacePicker";
import { settingsGet } from "../../settings";

interface NewAgentProps {
  onCreate: (input: { name: string; brief: string; engineId: string; faceIndex: number; homePath?: string }) => Promise<void>;
  onClose: () => void;
}

export function NewAgent({ onCreate, onClose }: NewAgentProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState("");
  const [brief, setBrief] = useState("");
  const [engineId, setEngineId] = useState("");
  const [faceIndex, setFaceIndex] = useState(0);
  const [homePath, setHomePath] = useState("");
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [drafting, setDrafting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const detect = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [detected, savedDefault] = await Promise.all([
        call("engines_detect"),
        settingsGet("default_engine").catch(() => null),
      ]);
      setEngines(detected);
      const usableIds = new Set(detected.filter((engine) => engine.status === "ready" && engine.supportsChat).map((engine) => engine.id));
      setEngineId((current) => current || (typeof savedDefault === "string" && savedDefault !== "auto" && usableIds.has(savedDefault) ? savedDefault : ""));
    } catch {
      setError("Could not check installed engines. Open Harbor on your desktop and try again.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    dialog.current?.showModal();
    void detect();
  }, [detect]);

  const usable = engines.filter((engine) => engine.status === "ready" && engine.supportsChat);
  const selectedEngine = usable.find((engine) => engine.id === engineId) ?? usable[0];
  const disabled = !name.trim() || !selectedEngine || loading || saving;

  async function draftWithAi() {
    if (drafting || saving) return;
    setDrafting(true);
    setError(null);
    try {
      const drafted = await call("agent_draft_with_ai", {
        hint: brief.trim() || name.trim() || "coding teammate",
      });
      if (drafted.name) setName(drafted.name);
      if (drafted.brief) setBrief(drafted.brief);
      if (drafted.engineId) setEngineId(drafted.engineId);
    } catch {
      setError("Could not draft a name and brief. You can still fill them in.");
    } finally {
      setDrafting(false);
    }
  }

  return (
    <dialog ref={dialog} className="harbor-dialog harbor-agent-dialog" aria-labelledby="new-agent-title"
      onCancel={(event) => { event.preventDefault(); if (!saving) onClose(); }}>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (disabled || !selectedEngine) return;
        setSaving(true);
        setError(null);
        void onCreate({ name: name.trim(), brief: brief.trim(), engineId: selectedEngine.id, faceIndex, homePath: homePath || undefined })
          .catch(() => setError("Your agent could not be created. Your draft is saved here. Please try again."))
          .finally(() => setSaving(false));
      }}>
        <span className="harbor-eyebrow">NEW TEAMMATE</span>
        <h2 id="new-agent-title">Make an agent your own.</h2>
        <p>A clear brief helps your agent understand the work.</p>
        <fieldset disabled={saving}>
          <label>Name<input autoFocus value={name} placeholder="e.g. Release partner" maxLength={40} onChange={(event) => setName(event.target.value)} /></label>
          <label>Brief<textarea value={brief} rows={3} placeholder="What should this agent help you with?" onChange={(event) => setBrief(event.target.value)} /></label>
          <label>Engine<select value={selectedEngine?.id ?? ""} disabled={loading || !usable.length} onChange={(event) => setEngineId(event.target.value)}>
            {usable.map((engine) => <option key={engine.id} value={engine.id}>{engine.displayName}</option>)}
            {!usable.length ? <option value="">{loading ? "Checking engines…" : "No chat engine ready"}</option> : null}
          </select></label>
          {!loading && !usable.length ? <div className="harbor-engine-help"><p>Install and sign in to an ACP-compatible engine such as OpenCode, then check again.</p><Button onClick={() => void detect()}>Check again</Button></div> : null}
          <label>Home folder
            <span className="harbor-home-picker">
              <input readOnly value={homePath} placeholder="Optional — engine working directory" />
              <Button type="button" onClick={() => {
                void call("workspace_pick_folder").then((path) => {
                  if (path) setHomePath(path);
                }).catch(() => setError("Could not choose a folder. Please try again."));
              }}>Choose…</Button>
            </span>
          </label>
          <p className="harbor-muted">Without a home folder the engine starts in a temporary directory. You can set this later in Settings.</p>
          <label>Choose a face</label>
          <FacePicker value={faceIndex} onChange={setFaceIndex} name={name || "Agent"} />
        </fieldset>
        {error ? <p className="harbor-inline-error" role="alert">{error}</p> : null}
        <div className="harbor-dialog-actions">
          <Button disabled={saving || drafting} onClick={() => void draftWithAi()}>{drafting ? "Drafting…" : "Create with AI"}</Button>
          <Button disabled={saving || drafting} onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="primary" disabled={disabled}>{saving ? "Creating…" : "Create agent"}</Button>
        </div>
      </form>
    </dialog>
  );
}
