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
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
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

it("creates a routine from the local form and persists it", async () => {
  render(<ChromeProvider value={chrome}><Routines /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Create your first routine" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Routine name" }), { target: { value: "Morning scan" } });
  fireEvent.change(screen.getByRole("textbox", { name: "What should the teammate do?" }), { target: { value: "Summarize the open decisions." } });
  fireEvent.click(screen.getByRole("button", { name: "Save routine" }));
  expect(await screen.findByRole("heading", { name: "Morning scan" })).toBeTruthy();
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("settings_set", expect.objectContaining({ key: "routines_local" })));
});
