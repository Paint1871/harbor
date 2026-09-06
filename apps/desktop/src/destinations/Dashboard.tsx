import { useChrome } from "../chrome/chrome-context";

const METRICS = [
  { label: "Agents", value: "4", detail: "1 working now" },
  { label: "Turns today", value: "128", detail: "31.6k tokens" },
  { label: "Credits", value: "9,684", detail: "PRO · resets in 12d" },
] as const;

const ACTIVITY = [
  { title: "X trend scout", detail: "Ranked 3 threads by signal", time: "4m", mode: "agent", live: true },
  { title: "Cold outreach operator", detail: "10 drafts held for approval", time: "17m", mode: "agent", live: false },
  { title: "Trend video strategist", detail: "Cut 3 shorts from the stream", time: "2h", mode: "agent", live: false },
  { title: "Harbor charter writer", detail: "Rewrote the authority section", time: "1d", mode: "chat", live: false },
] as const;

export function Dashboard() {
  const { onModeChange } = useChrome();
  return (
    <section className="harbor-destination-page harbor-dashboard" aria-label="Dashboard">
      <header className="harbor-dashboard-header">
        <h1>Dashboard</h1>
        <p>What is running, what finished, and where you are needed.</p>
      </header>

      <div className="harbor-dashboard-metrics" aria-label="Workspace metrics">
        {METRICS.map((metric) => (
          <article className="harbor-dashboard-metric" key={metric.label}>
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
            <small>{metric.detail}</small>
          </article>
        ))}
      </div>

      <div className="harbor-dashboard-activity" aria-label="Recent activity">
        {ACTIVITY.map((item) => (
          <button
            key={item.title}
            type="button"
            className="harbor-dashboard-activity-row"
            onClick={() => onModeChange(item.mode)}
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
              <strong>{item.title}</strong>
              <span>{item.detail}</span>
            </span>
            <span className="harbor-dashboard-activity-time">
              <i data-live={item.live} aria-hidden="true" />
              {item.time}
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}
