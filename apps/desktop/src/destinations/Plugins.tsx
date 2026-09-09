import { useCallback, useEffect, useState } from "react";
import { call } from "../ipc";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@harbor/ui/Button";
import type { AgentRecord, PluginApproval, PluginRow } from "@harbor/schema/commands";
import { ApprovalCard } from "./ApprovalCard";
import { PluginMark } from "./PluginMark";

interface DevicePayload {
  userCode?: string;
  verificationUri?: string;
  error?: string;
  connected?: boolean;
  id?: string;
}

const FALLBACK_DETAILS: Record<string, { description: string; category: string; authKind: "device" | "token" }> = {
  x: { description: "Post, reply, and read your timeline.", category: "Social", authKind: "token" },
  apollo: { description: "Find leads and enrich contacts mid-task.", category: "Sales", authKind: "token" },
  vidiq: { description: "Research keywords and read your channel stats.", category: "Video", authKind: "token" },
  higgsfield: { description: "Generate images and videos with your Higgsfield credits.", category: "Creative", authKind: "token" },
  fal: { description: "Generate images and videos with your fal.ai key.", category: "Creative", authKind: "token" },
  youtube: { description: "Read and manage the connected channel.", category: "Video", authKind: "token" },
  github: { description: "Read repos, issues, and pull requests with a GitHub PAT.", category: "Development", authKind: "token" },
  linear: { description: "Find, create, and update issues, projects, and comments.", category: "Planning", authKind: "token" },
  stripe: { description: "Read customers, invoices, and the catalog. Charges and refunds stay blocked.", category: "Commerce", authKind: "token" },
  cloudflare: { description: "Manage Workers, DNS, R2, and D1 on your account.", category: "Development", authKind: "token" },
  gmail: { description: "Read and send mail on the connected Google account.", category: "Communication", authKind: "token" },
  supabase: { description: "Inspect and change the builder's Supabase projects.", category: "Development", authKind: "token" },
  vercel: { description: "Manage projects, deployments, and domains.", category: "Development", authKind: "token" },
  shopify: { description: "Manage products, orders, and inventory.", category: "Commerce", authKind: "token" },
  slack: { description: "Read channels and post with builder approval.", category: "Communication", authKind: "token" },
  notion: { description: "Search, read, and update pages in your Notion workspace.", category: "Knowledge", authKind: "token" },
};

function details(row: PluginRow) {
  const fallback = FALLBACK_DETAILS[row.id] ?? { description: "A local connection for your workflow", category: "Other", authKind: "token" as const };
  return {
    description: row.description || fallback.description,
    category: row.category || fallback.category,
    authKind: row.authKind === "device" ? "device" as const : fallback.authKind,
  };
}

function displayName(row: PluginRow): string {
  return row.displayName || row.id;
}

function errorText(reason: unknown): string {
  const message = String(reason);
  return message.replace(/^Error:\s*/i, "").trim() || "The connection could not be updated.";
}

export function Plugins() {
  const [rows, setRows] = useState<PluginRow[]>([]);
  const [device, setDevice] = useState<DevicePayload | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentRecord[]>([]);
  const [grants, setGrants] = useState<Record<string, boolean>>({});
  const [approvals, setApprovals] = useState<PluginApproval[]>([]);
  const [setup, setSetup] = useState<PluginRow | null>(null);
  const [credential, setCredential] = useState("");
  const [accountLabel, setAccountLabel] = useState("");
  const [setupBusy, setSetupBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    let listedRows: PluginRow[] = [];
    try {
      const listed = await call("plugin_list");
      listedRows = Array.isArray(listed) ? listed : [];
      setRows(listedRows);
    } catch (reason) {
      setRows([]);
      setLoadError(`The connection catalog could not be loaded. ${errorText(reason)}`);
    }
    try {
      const listed = await call("agent_list");
      const listedAgents = Array.isArray(listed) ? listed : [];
      setAgents(listedAgents);
      const next: Record<string, boolean> = {};
      await Promise.all(
        listedAgents.map(async (agent) => {
          try {
            const agentGrants = await call("plugin_grants_list", { agentId: agent.id });
            for (const grant of agentGrants) next[`${agent.id}:${grant.pluginId}`] = grant.enabled;
          } catch {
            // A single agent grant should not blank the rest of the page.
          }
        }),
      );
      setGrants(next);
    } catch {
      setAgents([]);
      setGrants({});
    }
    try {
      const pending = await call("plugin_approvals_list");
      setApprovals(Array.isArray(pending) ? pending : []);
    } catch {
      setApprovals([]);
    }
    setLoading(false);
    return listedRows;
  }, []);

  useEffect(() => {
    void reload();
    let disposed = false;
    let stop: () => void = () => undefined;
    void listen<DevicePayload>("plugin_device", (event) => {
      setDevice(event.payload);
      if (event.payload.connected) {
        setNotice("The connection is ready.");
        void reload();
      }
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      stop();
    };
  }, [reload]);

  function beginConnect(row: PluginRow) {
    setDevice(null);
    setNotice(null);
    if (details(row).authKind === "token") {
      setSetup(row);
      setCredential("");
      setAccountLabel(row.accountLabel ?? "");
      return;
    }
    setBusy(row.id);
    setRows((current) => current.map((item) => item.id === row.id ? { ...item, status: "connecting" } : item));
    void call("plugin_connect", { id: row.id })
      .then(() => {
        setNotice(`${displayName(row)} is waiting for browser sign-in.`);
        return reload();
      })
      .catch((reason) => {
        setDevice({ error: errorText(reason) });
        return reload();
      })
      .finally(() => setBusy(null));
  }

  async function saveConnection() {
    if (!setup || !credential.trim() || setupBusy) return;
    const selected = setup;
    setSetupBusy(true);
    setDevice(null);
    setNotice(null);
    try {
      await call("plugin_configure", {
        id: selected.id,
        credential: credential.trim(),
        accountLabel: accountLabel.trim() || null,
      });
      setSetup(null);
      setCredential("");
      setAccountLabel("");
      await reload();
      setNotice(`${displayName(selected)} is connected.`);
    } catch (reason) {
      setDevice({ error: errorText(reason) });
    } finally {
      setSetupBusy(false);
    }
  }

  function disconnect(row: PluginRow) {
    setNotice(null);
    setBusy(row.id);
    void call("plugin_disconnect", { id: row.id })
      .then(() => reload())
      .then(() => {
        setDevice(null);
        setNotice(`${displayName(row)} was disconnected.`);
      })
      .catch((reason) => setDevice({ error: errorText(reason) }))
      .finally(() => setBusy(null));
  }

  function toggleGrant(agentId: string, pluginId: string, enabled: boolean) {
    const key = `${agentId}:${pluginId}`;
    setGrants((current) => ({ ...current, [key]: enabled }));
    setBusy(key);
    void call("plugin_set_agent_grant", { agentId, pluginId, enabled })
      .catch(() => setGrants((current) => ({ ...current, [key]: !enabled })))
      .finally(() => setBusy(null));
  }

  const needle = query.trim().toLowerCase();
  const visibleRows = rows.filter((row) => {
    if (!needle) return true;
    return `${displayName(row)} ${row.description} ${row.category}`.toLowerCase().includes(needle);
  });
  const connectedRows = rows.filter((row) => row.status === "connected");
  const connectedCount = connectedRows.length;

  return (
    <section className="harbor-destination-page harbor-connections-page" aria-label="Plugins">
      <div className="harbor-connections-heading">
        <div className="harbor-connections-heading-copy">
          <span className="harbor-eyebrow">CONNECTIONS</span>
          <h2>Plugins</h2>
          <p>Bring the tools you already use into your local workspace. Credentials stay in the OS keyring.</p>
        </div>
        <div className="harbor-connections-heading-tools">
          <label className="harbor-plugin-search">
            <span>Search plugins</span>
            <input aria-label="Search plugins" value={query} placeholder="Find a connection" onChange={(event) => setQuery(event.target.value)} />
          </label>
          <div className="harbor-connections-stat" aria-label={`${connectedCount} connections connected`}>
            <strong>{connectedCount}</strong>
            <span>connected</span>
          </div>
        </div>
      </div>

      {loadError ? <div className="harbor-connection-alert harbor-connection-alert-error" role="alert"><p>{loadError}</p><Button variant="ghost" onClick={() => void reload()}>Try again</Button></div> : null}
      {loading && !rows.length ? <div className="harbor-plugin-loading" role="status">Loading connections…</div> : null}
      {rows.length ? (
        <div className="harbor-plugin-list" aria-label="Plugin connections">
          {visibleRows.length ? visibleRows.map((row) => {
            const meta = details(row);
            const connected = row.status === "connected";
            const status = connected ? "Connected" : row.status === "connecting" ? "Connecting" : "Available";
            return (
              <article className="harbor-plugin-row" data-status={row.status} key={row.id}>
                <div className="harbor-plugin-row-main">
                  <PluginMark id={row.id} />
                  <div className="harbor-plugin-row-copy">
                    <h3>{displayName(row)}</h3>
                    <span>{row.description || meta.description}</span>
                  </div>
                </div>
                <div className="harbor-plugin-row-side">
                  <span className="harbor-plugin-category">{meta.category}</span>
                  <span className="harbor-plugin-account">
                    {connected ? (row.accountLabel || "Connected") : meta.authKind === "device" ? "Browser sign-in" : "Personal access token"}
                  </span>
                  <span className="harbor-plugin-status" data-status={row.status}>{status}</span>
                  {connected ? (
                    <Button variant="ghost" aria-label={`Disconnect ${displayName(row)}`} disabled={busy === row.id} onClick={() => disconnect(row)}>Disconnect</Button>
                  ) : (
                    <Button variant="primary" aria-label={`Connect ${displayName(row)}`} disabled={busy === row.id || row.status === "connecting"} onClick={() => beginConnect(row)}>
                      {busy === row.id || row.status === "connecting" ? "Connecting…" : "Connect"}
                    </Button>
                  )}
                </div>
              </article>
            );
          }) : <div className="harbor-plugin-no-results" role="status">No connections match “{query}”.</div>}
        </div>
      ) : (
        !loading ? <div className="harbor-destination-empty">
          <span className="harbor-eyebrow">NO CONNECTIONS</span>
          <p className="harbor-muted">Harbor could not load the local connection catalog. Try reopening this view.</p>
        </div> : null
      )}

      {device ? (
        <div className="harbor-connection-alert" role="status">
          <div>
            <span className="harbor-eyebrow">CONNECTION UPDATE</span>
            {device.userCode ? <p>Finish the browser sign-in with code <strong>{device.userCode}</strong> at {device.verificationUri}.</p> : null}
            {device.error ? <p>{device.error}</p> : null}
            {device.verificationUri ? <button type="button" className="harbor-connection-link" onClick={() => window.open(device.verificationUri, "_blank", "noopener,noreferrer")}>Open sign-in page</button> : null}
          </div>
          <Button variant="ghost" onClick={() => setDevice(null)}>Dismiss</Button>
        </div>
      ) : null}
      {notice ? <p className="harbor-destination-notice" role="status">{notice}</p> : null}

      {agents.length ? (
        <div className="harbor-connections-access">
          <div className="harbor-connections-section-heading">
            <div><span className="harbor-eyebrow">TEAMMATE ACCESS</span><h3>Choose who can use a connection.</h3></div>
            <span className="harbor-muted">Explicit grants</span>
          </div>
          {connectedRows.length ? (
            <div className="harbor-grant-list">
              {agents.map((agent) => (
                <div className="harbor-grant-row" key={agent.id}>
                  <strong>{agent.name}</strong>
                  <div className="harbor-grant-options">
                    {connectedRows.map((row) => {
                      const key = `${agent.id}:${row.id}`;
                      return (
                        <label key={row.id}>
                          <input
                            aria-label={row.id === "github" ? agent.name : `${agent.name} · ${displayName(row)}`}
                            type="checkbox"
                            checked={!!grants[key]}
                            disabled={busy === key}
                            onChange={(event) => toggleGrant(agent.id, row.id, event.target.checked)}
                          />
                          {displayName(row)}
                        </label>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="harbor-muted">Connect a tool above before granting it to a teammate.</p>
          )}
        </div>
      ) : null}

      {approvals.length ? (
        <div className="harbor-connection-approvals">
          <span className="harbor-eyebrow">PENDING</span>
          <h3>Approvals</h3>
          {approvals.map((row) => <ApprovalCard key={row.id} id={row.id} onResolved={() => void reload()} />)}
        </div>
      ) : (
        <div className="harbor-plugin-note">
          <span className="harbor-eyebrow">PERMISSIONS</span>
          <p>Connection is not a grant. Harbor asks before a teammate writes through a connected tool.</p>
        </div>
      )}

      {setup ? (
        <div className="harbor-connection-modal-backdrop" role="presentation" onMouseDown={() => { if (!setupBusy) setSetup(null); }}>
          <form
            className="harbor-connection-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="harbor-connection-modal-title"
            onSubmit={(event) => { event.preventDefault(); void saveConnection(); }}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="harbor-connection-modal-header">
              <PluginMark id={setup.id} />
              <div><span className="harbor-eyebrow">LOCAL CONNECTION</span><h3 id="harbor-connection-modal-title">Connect {displayName(setup)}</h3></div>
            </div>
            <p className="harbor-muted">Paste a provider credential. Harbor stores it locally in the OS keyring and never places it in SQLite or an engine environment.</p>
            <label>
              Account label <span>(optional)</span>
              <input value={accountLabel} maxLength={120} placeholder="Personal workspace" onChange={(event) => setAccountLabel(event.target.value)} />
            </label>
            <label>
              Credential
              <input aria-label="Credential" type="password" autoComplete="new-password" value={credential} placeholder="Paste a token or access key" onChange={(event) => setCredential(event.target.value)} />
            </label>
            <div className="harbor-connection-modal-actions">
              <Button type="button" variant="ghost" disabled={setupBusy} onClick={() => setSetup(null)}>Cancel</Button>
              <Button type="submit" variant="primary" disabled={setupBusy || !credential.trim()}>{setupBusy ? "Saving…" : "Save connection"}</Button>
            </div>
          </form>
        </div>
      ) : null}
    </section>
  );
}
