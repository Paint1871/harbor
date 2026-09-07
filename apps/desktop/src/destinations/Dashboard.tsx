import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AgentRecord, ThreadRecord, Workspace } from "@harbor/schema/commands";
import { useChrome } from "../chrome/chrome-context";

interface Counts {
  agents: AgentRecord[];
  workspaces: Workspace[];
  threads: ThreadRecord[];
}

const EMPTY: Counts = { agents: [], workspaces: [], threads: [] };

/**
 * 0.1.0 has no usage meter and no credit balance, so this page only counts what
 * the local database already holds. Nothing here is estimated or backfilled.
 */
export function Dashboard() {
  const { onModeChange } = useChrome();
  const [counts, setCounts] = useState<Counts>(EMPTY);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      invoke<AgentRecord[]>("agent_list"),
      invoke<Workspace[]>("workspace_list"),
      invoke<ThreadRecord[]>("thread_list", { workspaceId: null }),
    ])
      .then(([agents, workspaces, threads]) => {
        if (cancelled) return;
        setCounts({ agents, workspaces, threads });
        setFailed(false);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const metrics = [
    { label: "Agents", value: counts.agents.length, detail: "teammates on this machine" },
    { label: "Workspaces", value: counts.workspaces.length, detail: "folders you added" },
    { label: "Threads", value: counts.threads.length, detail: "folder-scoped chats" },
  ];

  const unread = counts.threads.filter((thread) => thread.unread);
  const recent = [...counts.threads].sort((a, b) => Number(b.pinned) - Number(a.pinned)).slice(0, 6);

  return (
    <section className="harbor-destination-page harbor-dashboard" aria-label="Dashboard">
      <header className="harbor-dashboard-header">
        <h1>Dashboard</h1>
        <p>What is running, what finished, and where you are needed.</p>
      </header>

      {failed ? (
        <p className="harbor-dashboard-error" role="status">
          Harbor could not read the local database.
        </p>
      ) : null}

      <div className="harbor-dashboard-metrics" aria-label="Workspace metrics">
        {metrics.map((metric) => (
          <article className="harbor-dashboard-metric" key={metric.label}>
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
            <small>{metric.detail}</small>
          </article>
        ))}
      </div>

      <div className="harbor-dashboard-activity" aria-label="Threads">
        {recent.length === 0 ? (
          <p className="harbor-dashboard-empty">
            No threads yet. Open Chat, pick a folder, and start one.
          </p>
        ) : (
          recent.map((thread) => (
            <button
              key={thread.id}
              type="button"
              className="harbor-dashboard-activity-row"
              onClick={() => onModeChange("chat")}
            >
              <span className="harbor-dashboard-activity-mark" aria-hidden="true">
                <svg width="16" height="16" viewBox="0 0 16 16">
                  <circle cx="3.25" cy="8" r="1.35" fill="currentColor" />
                  <circle cx="12.75" cy="4" r="1.35" fill="currentColor" />
                  <circle cx="12.75" cy="12" r="1.35" fill="currentColor" />
                  <path d="M4.45 7.35 11.45 4.65M4.45 8.65l7 2.7" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" />
                </svg>
              </span>
              <span className="harbor-dashboard-activity-copy">
                <strong>{thread.title}</strong>
                <span>{thread.engineId}</span>
              </span>
              <span className="harbor-dashboard-activity-time">
                <i data-live={thread.unread} aria-hidden="true" />
                {thread.unread ? "unread" : thread.pinned ? "pinned" : ""}
              </span>
            </button>
          ))
        )}
      </div>

      {unread.length > 0 ? (
        <p className="harbor-dashboard-empty">
          {unread.length} thread{unread.length === 1 ? "" : "s"} waiting on you.
        </p>
      ) : null}
    </section>
  );
}
