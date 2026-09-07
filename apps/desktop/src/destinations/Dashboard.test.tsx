// @vitest-environment jsdom
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Dashboard } from "./Dashboard";
import { ChromeProvider, type ChromeValue } from "../chrome/chrome-context";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const chrome: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Ada",
  destination: "dashboard",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
};

describe("Dashboard", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  it("counts real rows and never shows a credit meter", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "agent_list") return Promise.resolve([{ id: "a1" }, { id: "a2" }]);
      if (command === "workspace_list") return Promise.resolve([{ id: "w1" }]);
      if (command === "thread_list") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(
      <ChromeProvider value={chrome}>
        <Dashboard />
      </ChromeProvider>,
    );
    await waitFor(() => expect(screen.getByText("2")).toBeTruthy());
    expect(screen.getByText("Agents")).toBeTruthy();
    expect(screen.queryByText("Credits")).toBeNull();
    expect(screen.getByText(/No threads yet/)).toBeTruthy();
  });
});