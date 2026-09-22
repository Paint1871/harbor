import { call } from "../ipc";
import { pendingPromptKey } from "../pending-prompt";
import { settingsGet, settingsSet } from "../settings";

export type Frequency = "weekdays" | "weekly" | "manual";

export interface Routine {
  id: string;
  name: string;
  brief: string;
  frequency: Frequency;
  agentId: string;
  enabled: boolean;
  /** Epoch ms of the last scheduled fire. Manual runs do not touch it. */
  lastFiredAt?: number;
}

export const ROUTINES_KEY = "routines_local";
export { PENDING_PROMPT_PREFIX } from "../pending-prompt";

export function readRoutines(value: unknown): Routine[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is Routine => {
    if (!item || typeof item !== "object") return false;
    const row = item as Partial<Routine>;
    return typeof row.id === "string"
      && typeof row.name === "string"
      && typeof row.brief === "string"
      && (row.frequency === "weekdays" || row.frequency === "weekly" || row.frequency === "manual")
      && typeof row.agentId === "string"
      && typeof row.enabled === "boolean";
  });
}

function startOfDay(now: Date): number {
  const day = new Date(now);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/** Local Monday 00:00 of `now`'s week. */
function startOfWeek(now: Date): number {
  const day = new Date(now);
  day.setDate(day.getDate() - ((day.getDay() + 6) % 7));
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

/**
 * Start of the schedule window `now` falls into, or null when the frequency
 * cannot fire at all right now (manual routines, weekends for "weekdays").
 */
export function routinePeriodStart(frequency: Frequency, now: Date): number | null {
  switch (frequency) {
    case "weekdays": {
      const day = now.getDay();
      if (day === 0 || day === 6) return null;
      return startOfDay(now);
    }
    case "weekly":
      return startOfWeek(now);
    case "manual":
      return null;
  }
}

export function isRoutineDue(routine: Routine, now: Date): boolean {
  if (!routine.enabled) return false;
  const start = routinePeriodStart(routine.frequency, now);
  return start !== null && (routine.lastFiredAt ?? 0) < start;
}

/**
 * Fires every due routine once: a chat is prepared with the brief waiting in
 * the composer and an inbox event points back to it. The engine only starts
 * when the builder reviews and sends, so a schedule can never burn tokens on
 * its own. A routine that fails to fire is still stamped so it cannot loop.
 */
export async function runDueRoutines(now: Date = new Date()): Promise<number> {
  const routines = readRoutines(await settingsGet(ROUTINES_KEY));
  const due = routines.filter((routine) => isRoutineDue(routine, now));
  if (!due.length) return 0;
  const firedAt = now.getTime();
  for (const routine of due) {
    try {
      const chat = await call("agent_chat_create", { agentId: routine.agentId });
      await call("agent_chat_rename", { chatId: chat.id, title: routine.name }).catch(() => undefined);
      await settingsSet(pendingPromptKey(chat.id), routine.brief);
      await call("notification_record", {
        kind: "routine",
        title: `${routine.name} is ready`,
        body: "The brief is waiting in a new chat. Review it, then send.",
        mode: "agent",
        sessionRef: chat.id,
      }).catch(() => undefined);
    } catch {
      // Stamped below either way: a deleted agent must not re-fire every tick.
      // But a silent miss is worse — tell the builder the schedule broke.
      await call("notification_record", {
        kind: "routine",
        title: `${routine.name} could not run`,
        body: "The linked teammate may have been removed. Open Routines to check it.",
        mode: "agent",
      }).catch(() => undefined);
    }
  }
  const fired = new Set(due.map((routine) => routine.id));
  // Re-read before writing back: an edit made while chats were being prepared
  // must not be rolled back by the stale snapshot taken above.
  const latest = readRoutines(await settingsGet(ROUTINES_KEY));
  const next = latest.map((routine) =>
    fired.has(routine.id) ? { ...routine, lastFiredAt: firedAt } : routine);
  await settingsSet(ROUTINES_KEY, next);
  return due.length;
}
