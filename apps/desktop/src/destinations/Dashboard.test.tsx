// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Dashboard } from "./Dashboard";
import { ChromeProvider, type ChromeValue } from "../chrome/chrome-context";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const chrome = (over: Partial<ChromeValue> = {}): ChromeValue => ({
  mode: "agent",
  theme: "black",
  profileName: "Ada",
  destination: "dashboard",
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
  ...over,
});

describe("Dashboard", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  it("counts real rows and never shows a credit meter", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "agent_list") return Promise.resolve([{ id: "a1" }, { id: "a2" }]);
      if (command === "workspace_list") return Promise.resolve([{ id: "w1" }]);
      if (command === "thread_list_all") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(
      <ChromeProvider value={chrome()}>
        <Dashboard />
      </ChromeProvider>,
    );
    await waitFor(() => expect(screen.getByText("2")).toBeTruthy());
    expect(screen.getByText("Agents")).toBeTruthy();
    expect(screen.queryByText("Credits")).toBeNull();
    expect(screen.getByText(/No threads yet/)).toBeTruthy();
  });

  it("opens the chat thread a row came from", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "agent_list") return Promise.resolve([]);
      if (command === "workspace_list") return Promise.resolve([]);
      if (command === "thread_list_all") {
        return Promise.resolve([
          { id: "thread-1", workspaceId: "ws-1", engineId: "opencode", title: "Launch", pinned: false, unread: true },
        ]);
      }
      return Promise.resolve([]);
    });
    const onOpenSession = vi.fn();
    render(
      <ChromeProvider value={chrome({ onOpenSession })}>
        <Dashboard />
      </ChromeProvider>,
    );
    await waitFor(() => expect(screen.getByText("Launch")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: /Launch/ }));
    expect(onOpenSession).toHaveBeenCalledWith({
      mode: "chat",
      sessionRef: "thread-1",
      workspaceId: "ws-1",
    });
  });
});
