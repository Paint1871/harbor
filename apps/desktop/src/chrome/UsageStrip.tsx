import { useCallback, useEffect, useState } from "react";
import { Button } from "@harbor/ui/Button";
import { call } from "../ipc";
import { EngineMark } from "./EngineMark";
import type { EngineUsage, UsageWindow } from "@harbor/schema/commands";

/** `5h` and `Weekly` are the windows the CLIs actually use; the rest read as hours. */
export function windowName(minutes: number): string {
  if (minutes >= 10080) return "Weekly";
  if (minutes >= 1440) return `${Math.round(minutes / 1440)}d`;
  return `${Math.round(minutes / 60)}h`;
}

/** Coarse on purpose: a quota that resets in four days does not need minutes. */
export function untilReset(resetsAt: number, now: number): string {
  const seconds = Math.max(0, resetsAt - now);
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  if (days) return `${days}d ${hours}h`;
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function Meter({ window: limit, now }: { window: UsageWindow; now: number }) {
  const used = Math.min(100, Math.max(0, limit.usedPercent));
  return (
    <div className="harbor-usage-meter">
      <span
        className="harbor-usage-track"
        role="meter"
        aria-label={`${windowName(limit.windowMinutes)} limit`}
        aria-valuenow={Math.round(used)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuetext={`${Math.round(used)}% used, resets in ${untilReset(limit.resetsAt, now)}`}
      >
        <span className="harbor-usage-fill" data-level={used >= 90 ? "spent" : used >= 70 ? "high" : "fine"} style={{ width: `${used}%` }} />
      </span>
      <span className="harbor-usage-read">
        <strong>{Math.round(used)}% used</strong>
        <span>{untilReset(limit.resetsAt, now)}</span>
      </span>
    </div>
  );
}

/**
 * What the hosted CLIs report about their own plans. Harbor has no account and
 * asks no vendor anything, so an engine shows up only while it writes its
 * limits to disk — and the section stays out of the way entirely when none do,
 * rather than sitting in the rail explaining its own absence.
 */
export function UsageStrip() {
  const [engines, setEngines] = useState<EngineUsage[]>([]);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      setEngines(await call("engine_usage"));
      setNow(Math.floor(Date.now() / 1000));
    } catch {
      setEngines([]);
    } finally {
      setBusy(false);
    }
  }, []);

  // Usage arrives while Harbor is already open — a CLI runs, or the builder
  // connects the bridge — and the manual button is inside the section that
  // hides itself when there is nothing, so reading only on mount would strand
  // the strip empty with no way back. Re-read on a timer and whenever the
  // window comes forward; the reads are bounded and touch no network.
  useEffect(() => {
    void refresh();
    const poll = setInterval(() => void refresh(), 60_000);
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(poll);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh]);

  // The countdown moves between reads, so the clock ticks on its own.
  useEffect(() => {
    const tick = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 30_000);
    return () => clearInterval(tick);
  }, []);

  const live = engines.filter((engine) => engine.windows.some((limit) => limit.resetsAt > now));
  if (!live.length) return null;

  return (
    <section className="harbor-usage" aria-label="Usage">
      <div className="harbor-usage-heading">
        <span className="harbor-eyebrow">USAGE</span>
        <Button size="icon" variant="ghost" aria-label="Refresh usage" title="Refresh usage" disabled={busy} onClick={() => void refresh()}>
          <svg width="13" height="13" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M13.2 8a5.2 5.2 0 1 1-1.6-3.75M13.4 2.4v2.9h-2.9" fill="none" stroke="currentColor" strokeWidth="1.35" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </Button>
      </div>
      {live.map((engine) => (
        <div className="harbor-usage-engine" key={engine.engineId}>
          <span className="harbor-usage-engine-name">
            <EngineMark engineId={engine.engineId} label={engine.displayName} size={13} />
            {engine.displayName}
          </span>
          {engine.windows
            .filter((limit) => limit.resetsAt > now)
            .map((limit) => <Meter key={limit.windowMinutes} window={limit} now={now} />)}
        </div>
      ))}
    </section>
  );
}
