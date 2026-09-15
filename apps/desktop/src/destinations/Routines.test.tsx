// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AgentRecord } from "@harbor/schema/commands";
import { ChromeProvider, type ChromeValue } from "../chrome/chrome-context";
import { Routines } from "./Routines";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const agent: AgentRecord = {
  id: "agent-1",
  name: "Release manager",
  brief: "Ship the launch",
  engineId: "opencode",
  faceIndex: 0,
  pinned: false,
};

const chrome: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Local",
  destination: "routines",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onOpenSession: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
  registerNewThread: () => undefined,
  registerNewAgentChat: () => undefined,
  registerCodeWorkspace: () => undefined,
};

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation((command: string) => {
    if (command === "settings_get") return Promise.resolve(null);
    if (command === "agent_list") return Promise.resolve([agent]);
    return Promise.resolve();
  });
});
afterEach(cleanup);

const morning = {
  id: "r1",
  name: "Morning scan",
  brief: "Summarize the open decisions.",
  frequency: "weekdays",
  agentId: "agent-1",
  enabled: true,
};
const weekly = {
  id: "r2",
  name: "Weekly notes",
  brief: "Collect release notes.",
  frequency: "weekly",
  agentId: "agent-1",
  enabled: true,
};

function seedRoutines(rows: unknown[]) {
  invoke.mockImplementation((command: string) => {
    if (command === "settings_get") return Promise.resolve(rows);
    if (command === "agent_list") return Promise.resolve([agent]);
    if (command === "agent_chat_create") {
      return Promise.resolve({ id: "chat-42", agentId: "agent-1", title: "New chat", status: "idle" });
    }
    return Promise.resolve();
  });
}

it("creates a routine from the local form and persists it", async () => {
  render(<ChromeProvider value={chrome}><Routines /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Create your first routine" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Routine name" }), { target: { value: "Morning scan" } });
  fireEvent.change(screen.getByRole("textbox", { name: "What should the teammate do?" }), { target: { value: "Summarize the open decisions." } });
  fireEvent.click(screen.getByRole("button", { name: "Save routine" }));
  expect(await screen.findByRole("heading", { name: "Morning scan" })).toBeTruthy();
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("settings_set", expect.objectContaining({ key: "routines_local" })));
});

it("filters the list and hides a non-matching name", async () => {
  seedRoutines([morning, weekly]);
  render(<ChromeProvider value={chrome}><Routines /></ChromeProvider>);
  expect(await screen.findByRole("heading", { name: "Morning scan" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Weekly notes" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search routines" }), { target: { value: "morning" } });
  expect(screen.getByRole("heading", { name: "Morning scan" })).toBeTruthy();
  expect(screen.queryByRole("heading", { name: "Weekly notes" })).toBeNull();
});

it("opens the created chat when a routine is run", async () => {
  const onOpenSession = vi.fn();
  seedRoutines([morning]);
  render(<ChromeProvider value={{ ...chrome, onOpenSession }}><Routines /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Run now" }));
  await waitFor(() => {
    expect(invoke).toHaveBeenCalledWith("agent_chat_create", { agentId: "agent-1" });
    expect(onOpenSession).toHaveBeenCalledWith({ mode: "agent", sessionRef: "chat-42" });
  });
});
