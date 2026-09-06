import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import type { AgentChat, AgentRecord, Memory, Place, PluginGrant, PluginRow, SearchHit } from "@harbor/schema/commands";
import { FacePicker } from "./FacePicker";

interface GearPanelProps {
  agent: AgentRecord;
  chats?: AgentChat[];
  onAgentChange?: (agent: AgentRecord) => void;
  onOpenChat?: (chatId: string) => void;
  onOpenSkills?: () => void;
  onOpenPlugins?: () => void;
}

const FALLBACK_PLUGIN: PluginRow = {
  id: "github",
  displayName: "GitHub",
  status: "available",
  accountLabel: null,
  description: "Repositories, issues, and pull requests",
  category: "Development",
  authKind: "device",
};

function recency(createdAt: number): string {
  const delta = Date.now() / 1000 - createdAt;
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h`;
  return `${Math.floor(delta / 86400)}d`;
}

export function GearPanel({ agent, chats = [], onAgentChange, onOpenChat, onOpenSkills, onOpenPlugins }: GearPanelProps) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [fact, setFact] = useState("");
  const [places, setPlaces] = useState<Place[]>([]);
  const [faceIndex, setFaceIndex] = useState(agent.faceIndex);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [plugins, setPlugins] = useState<PluginRow[]>([FALLBACK_PLUGIN]);
  const [pluginGrants, setPluginGrants] = useState<Record<string, boolean>>({});

  useEffect(() => {
    setFaceIndex(agent.faceIndex);
    setError(null);
    setQuery("");
    setHits([]);
    void invoke<Memory[]>("memory_list", { agentId: agent.id })
      .then(setMemories)
      .catch(() => setMemories([]));
    void invoke<Place[]>("places_list", { agentId: agent.id })
      .then(setPlaces)
      .catch(() => setPlaces([]));
    void invoke<PluginRow[]>("plugin_list")
      .then((listed) => setPlugins(Array.isArray(listed) && listed.length ? listed : [FALLBACK_PLUGIN]))
      .catch(() => setPlugins([FALLBACK_PLUGIN]));
    void invoke<PluginGrant[]>("plugin_grants_list", { agentId: agent.id })
      .then((grants) => setPluginGrants(Object.fromEntries(grants.map((grant) => [grant.pluginId, grant.enabled]))))
      .catch(() => setPluginGrants({}));
  }, [agent.id, agent.faceIndex]);

  async function addMemory() {
    const body = fact.trim();
    if (!body || busy) return;
    setBusy(true);
    setError(null);
    try {
      const created = await invoke<Memory>("memory_upsert", { agentId: agent.id, body });
      setMemories((current) => [...current, created]);
      setFact("");
    } catch {
      setError("That fact could not be saved. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function removeMemory(id: string) {
    setError(null);
    try {
      await invoke("memory_delete", { id });
      setMemories((current) => current.filter((item) => item.id !== id));
    } catch {
      setError("That fact could not be removed. Please try again.");
    }
  }

  async function grantPlace() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const path = await invoke<string | null>("workspace_pick_folder");
      if (!path) return;
      await invoke("places_grant", { agentId: agent.id, path });
      setPlaces(await invoke<Place[]>("places_list", { agentId: agent.id }));
    } catch (reason) {
      setError(`That folder could not be granted. ${String(reason)}`);
    } finally {
      setBusy(false);
    }
  }

  async function revokePlace(id: string) {
    setError(null);
    try {
      await invoke("places_revoke", { id });
      setPlaces((current) => current.filter((row) => row.id !== id));
    } catch {
      setError("That folder could not be revoked. Please try again.");
    }
  }

  async function searchChats() {
    const text = query.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    try {
      setHits(await invoke<SearchHit[]>("session_search", { agentId: agent.id, query: text }));
    } catch {
      setError("Search could not run. Please try again.");
      setHits([]);
    } finally {
      setBusy(false);
    }
  }

  async function togglePlugin(pluginId: string, enabled: boolean) {
    setPluginGrants((current) => ({ ...current, [pluginId]: enabled }));
    setError(null);
    try {
      await invoke("plugin_set_agent_grant", { agentId: agent.id, pluginId, enabled });
    } catch {
      setPluginGrants((current) => ({ ...current, [pluginId]: !enabled }));
      setError("Plugin access could not be updated. Please try again.");
    }
  }

  async function chooseFace(index: number) {
    const previous = faceIndex;
    setFaceIndex(index);
    setError(null);
    try {
      await invoke("agent_update", { input: { id: agent.id, faceIndex: index } });
      await invoke<string>("face_preview", { agentId: agent.id, faceIndex: index });
      onAgentChange?.({ ...agent, faceIndex: index });
    } catch {
      setFaceIndex(previous);
      setError("The face could not be updated. Please try again.");
    }
  }

  return (
    <aside className="harbor-gear" aria-label="Agent gear">
      <div className="harbor-gear-intro">
        <span className="harbor-eyebrow">AGENT SETTINGS</span>
        <h3>Shape {agent.name}</h3>
        <p>Keep this teammate grounded with a small set of facts, trusted folders, and a clear identity.</p>
      </div>

      <section className="harbor-gear-section">
        <div className="harbor-gear-section-heading">
          <div><span className="harbor-eyebrow">HISTORY</span><h3>Chats</h3></div>
          <span className="harbor-chip">{chats.length}</span>
        </div>
        {chats.length ? (
          <div className="harbor-gear-list">
            {chats.map((chat) => <button key={chat.id} type="button" onClick={() => onOpenChat?.(chat.id)}>{chat.title}</button>)}
          </div>
        ) : <p className="harbor-muted">Tabs for this teammate will appear here.</p>}
        <form className="harbor-gear-form" onSubmit={(event) => { event.preventDefault(); void searchChats(); }}>
          <label>
            Search chats
            <input value={query} placeholder="Find a past message" onChange={(event) => setQuery(event.target.value)} />
          </label>
          <Button type="submit" disabled={busy || !query.trim()}>Search</Button>
        </form>
        {hits.length ? (
          <div className="harbor-gear-search-results" aria-label="Search results">
            {hits.map((hit) => (
              <button key={`${hit.chat_id}-${hit.created_at}`} type="button" onClick={() => onOpenChat?.(hit.chat_id)}>
                <span>{hit.prose}</span><small>{recency(hit.created_at)}</small>
              </button>
            ))}
          </div>
        ) : null}
      </section>

      <section className="harbor-gear-section">
        <div className="harbor-gear-section-heading">
          <div><span className="harbor-eyebrow">CONTEXT</span><h3>Memory</h3></div>
          <span className="harbor-chip">Facts</span>
        </div>
        <p className="harbor-muted">Facts only — never secrets.</p>
        {memories.length ? (
          <div className="harbor-gear-list harbor-memory-list">
            {memories.map((item) => (
              <div key={item.id} className="harbor-gear-list-row"><span>{item.body}</span><Button variant="ghost" onClick={() => void removeMemory(item.id)}>Remove</Button></div>
            ))}
          </div>
        ) : <p className="harbor-muted">No remembered facts yet.</p>}
        <form className="harbor-gear-form" onSubmit={(event) => { event.preventDefault(); void addMemory(); }}>
          <label>
            Add a fact
            <input value={fact} maxLength={240} placeholder="Prefers tests before merge" onChange={(event) => setFact(event.target.value)} />
          </label>
          <Button type="submit" disabled={busy || !fact.trim()}>Remember</Button>
        </form>
      </section>

      <section className="harbor-gear-section">
        <div className="harbor-gear-section-heading">
          <div><span className="harbor-eyebrow">ACCESS</span><h3>Places</h3></div>
          <span className="harbor-chip">{places.length}</span>
        </div>
        <p className="harbor-muted">Folders this agent may read. Home is always included.</p>
        {places.length ? (
          <div className="harbor-gear-list">
            {places.map((row) => (
              <div key={row.id} className="harbor-gear-list-row"><span className="harbor-path">{row.path}</span><Button variant="ghost" onClick={() => void revokePlace(row.id)}>Revoke</Button></div>
            ))}
          </div>
        ) : <p className="harbor-muted">No extra folders granted.</p>}
        <Button disabled={busy} onClick={() => void grantPlace()}>Grant folder</Button>
      </section>

      <section className="harbor-gear-section">
        <div className="harbor-gear-section-heading"><div><span className="harbor-eyebrow">TOOLS</span><h3>Plugins</h3></div></div>
        {plugins.filter((plugin) => plugin.status === "connected").length ? (
          <div className="harbor-gear-plugin-list">
            {plugins.filter((plugin) => plugin.status === "connected").map((plugin) => (
              <label className="harbor-check-row" key={plugin.id}>
                <input
                  aria-label={plugin.id === "github" ? "GitHub" : plugin.displayName}
                  type="checkbox"
                  checked={!!pluginGrants[plugin.id]}
                  onChange={(event) => void togglePlugin(plugin.id, event.target.checked)}
                />
                <span><strong>{plugin.displayName}</strong><small>Allow this agent to use the connected plugin.</small></span>
              </label>
            ))}
          </div>
        ) : <p className="harbor-muted">Connect a plugin before granting it to this teammate.</p>}
        {onOpenPlugins ? <Button variant="ghost" onClick={onOpenPlugins}>Manage connections <span aria-hidden="true">↗</span></Button> : null}
        <div className="harbor-gear-subsection">
          <span className="harbor-eyebrow">SKILLS</span>
          <p className="harbor-muted">Reusable local playbooks for recurring work.</p>
          <Button variant="ghost" onClick={onOpenSkills}>Browse skills <span aria-hidden="true">↗</span></Button>
        </div>
      </section>

      <section className="harbor-gear-section">
        <div className="harbor-gear-section-heading"><div><span className="harbor-eyebrow">IDENTITY</span><h3>Settings</h3></div></div>
        <div className="harbor-gear-engine"><span>Engine</span><strong>{agent.engineId}</strong></div>
        <label className="harbor-face-label">Choose a face</label>
        <FacePicker name={agent.name} value={faceIndex} onChange={(index) => void chooseFace(index)} />
      </section>
      {error ? <p className="harbor-inline-error" role="alert">{error}</p> : null}
    </aside>
  );
}
