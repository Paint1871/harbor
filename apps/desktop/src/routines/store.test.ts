import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  isRoutineDue,
  readRoutines,
  routinePeriodStart,
  runDueRoutines,
  type Routine,
} from "./store";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function routine(partial: Partial<Routine> = {}): Routine {
  return {
    id: "r1",
    name: "Morning scan",
    brief: "Summarize the open decisions.",
    frequency: "weekdays",
    agentId: "agent-1",
    enabled: true,
    ...partial,
  };
}

const monday = new Date(2026, 8, 21, 9, 30); // 2026-09-21 is a Monday
const saturday = new Date(2026, 8, 26, 9, 30);
const nextMonday = new Date(2026, 8, 28, 9, 30);

describe("routinePeriodStart", () => {
  it("fires once per weekday and never on weekends or manual schedules", () => {
    const today = new Date(monday); today.setHours(0, 0, 0, 0);
    expect(routinePeriodStart("weekdays", monday)).toBe(today.getTime());
    expect(routinePeriodStart("weekdays", saturday)).toBeNull();
    expect(routinePeriodStart("manual", monday)).toBeNull();
  });

  it("weekly anchors to local Monday midnight", () => {
    const weekStart = new Date(2026, 8, 21, 0, 0, 0, 0);
    expect(routinePeriodStart("weekly", monday)).toBe(weekStart.getTime());
    expect(routinePeriodStart("weekly", saturday)).toBe(weekStart.getTime());
    expect(routinePeriodStart("weekly", nextMonday)).toBe(
      new Date(2026, 8, 28, 0, 0, 0, 0).getTime(),
    );
  });
});

describe("isRoutineDue", () => {
  it("is due once per period: un-fired, then quiet until the next period", () => {
    const r = routine();
    expect(isRoutineDue(r, monday)).toBe(true);
    const fired = { ...r, lastFiredAt: monday.getTime() };
    expect(isRoutineDue(fired, new Date(2026, 8, 21, 18, 0))).toBe(false);
    expect(isRoutineDue(fired, new Date(2026, 8, 22, 8, 0))).toBe(true);
  });

  it("weekly fires once per week, from any weekday", () => {
    const r = routine({ frequency: "weekly" });
    expect(isRoutineDue(r, saturday)).toBe(true);
    const fired = { ...r, lastFiredAt: saturday.getTime() };
    expect(isRoutineDue(fired, new Date(2026, 8, 27, 12, 0))).toBe(false);
    expect(isRoutineDue(fired, nextMonday)).toBe(true);
  });

  it("disabled and manual routines never fire", () => {
    expect(isRoutineDue(routine({ enabled: false }), monday)).toBe(false);
    expect(isRoutineDue(routine({ frequency: "manual" }), monday)).toBe(false);
  });
});

describe("readRoutines", () => {
  it("keeps valid rows, tolerates lastFiredAt, drops junk", () => {
    const rows = readRoutines([
      routine({ lastFiredAt: 123 }),
      { id: "x", name: 1 },
      "garbage",
      null,
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0].lastFiredAt).toBe(123);
  });
});

describe("runDueRoutines", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "settings_get") return Promise.resolve([routine()]);
      if (command === "agent_chat_create") {
        return Promise.resolve({ id: "chat-1", agentId: "agent-1", title: "New chat", status: "idle" });
      }
      return Promise.resolve();
    });
  });

  it("prepares a chat with the brief and records an inbox event", async () => {
    const fired = await runDueRoutines(monday);
    expect(fired).toBe(1);
    expect(invoke).toHaveBeenCalledWith("agent_chat_create", { agentId: "agent-1" });
    expect(invoke).toHaveBeenCalledWith("agent_chat_rename", {
      chatId: "chat-1",
      title: "Morning scan",
    });
    expect(invoke).toHaveBeenCalledWith("settings_set", {
      key: "pending_agent_prompt:chat-1",
      value: "Summarize the open decisions.",
    });
    expect(invoke).toHaveBeenCalledWith("notification_record", {
      kind: "routine",
      title: "Morning scan is ready",
      body: "The brief is waiting in a new chat. Review it, then send.",
      mode: "agent",
      sessionRef: "chat-1",
    });
    expect(invoke).toHaveBeenCalledWith("settings_set", {
      key: "routines_local",
      value: [expect.objectContaining({ id: "r1", lastFiredAt: monday.getTime() })],
    });
  });

  it("does nothing when nothing is due", async () => {
    const fired = await runDueRoutines(saturday);
    expect(fired).toBe(0);
    expect(invoke).not.toHaveBeenCalledWith("agent_chat_create", expect.anything());
  });

  it("stamps a routine even when its agent is gone so it cannot loop", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "settings_get") return Promise.resolve([routine()]);
      if (command === "agent_chat_create") return Promise.reject(new Error("agent not found"));
      return Promise.resolve();
    });
    const fired = await runDueRoutines(monday);
    expect(fired).toBe(1);
    expect(invoke).toHaveBeenCalledWith("settings_set", {
      key: "routines_local",
      value: [expect.objectContaining({ lastFiredAt: monday.getTime() })],
    });
    expect(invoke).toHaveBeenCalledWith("notification_record", {
      kind: "routine",
      title: "Morning scan could not run",
      body: "The linked teammate may have been removed. Open Routines to check it.",
      mode: "agent",
    });
  });

  it("a second concurrent tick shares the same run instead of double-firing", async () => {
    let resolveChat: ((value: unknown) => void) | undefined;
    invoke.mockImplementation((command: string) => {
      if (command === "settings_get") return Promise.resolve([routine()]);
      if (command === "agent_chat_create") {
        return new Promise((resolve) => { resolveChat = resolve; });
      }
      return Promise.resolve();
    });
    const first = runDueRoutines(monday);
    const second = runDueRoutines(monday);
    expect(second).toBe(first);
    await vi.waitFor(() => { expect(resolveChat).toBeDefined(); });
    resolveChat?.({ id: "chat-1", agentId: "agent-1" });
    await Promise.all([first, second]);
    const creates = invoke.mock.calls.filter(([command]) => command === "agent_chat_create");
    expect(creates).toHaveLength(1);
  });
});
