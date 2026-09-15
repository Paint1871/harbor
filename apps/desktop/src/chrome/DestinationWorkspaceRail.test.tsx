// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Workspace } from "@harbor/schema/commands";
import { DestinationWorkspaceRail } from "./DestinationWorkspaceRail";
import { ChromeProvider, type ChromeValue } from "./chrome-context";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const chrome: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Local",
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
};

const launch: Workspace = { id: "a", folder: "/tmp/launch", title: "Launch work", pinned: false };
const notes: Workspace = { id: "b", folder: "/Users/x/notes-repo", title: "Release notes", pinned: false };

function renderRail() {
  render(
    <ChromeProvider value={chrome}>
      <DestinationWorkspaceRail />
    </ChromeProvider>,
  );
}

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "workspace_list") return Promise.resolve([launch, notes]);
    if (command === "layout_restore") return Promise.resolve([]);
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

it("hides a non-matching folder and restores every row when the query is cleared", async () => {
  renderRail();
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("workspace_list"));
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("layout_restore"));
  expect(await screen.findByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();
  expect(screen.getByRole("textbox", { name: "Search workspaces" })).toBeTruthy();
  expect(screen.getByPlaceholderText("Find a workspace")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add to Workspaces" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "launch" } });
  expect(screen.getByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Release notes" })).toBeNull();
  expect(screen.getByRole("button", { name: "Add to Workspaces" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "" } });
  expect(screen.getByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();
});

it("keeps expand, rename, and remove on a visible row and shows empty-match copy", async () => {
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "workspace_list") return Promise.resolve([launch, notes]);
    if (command === "layout_restore") {
      return Promise.resolve([
        {
          id: "tab-a",
          workspaceId: "a",
          layout: { type: "leaf", paneId: "term-1" },
          panes: [{ id: "term-1", kind: "terminal", paused: true, engineId: "shell" }],
        },
      ]);
    }
    return Promise.resolve([]);
  });
  renderRail();
  expect(await screen.findByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(await screen.findByRole("list", { name: "Launch work panes" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "notes-repo" } });
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Launch work" })).toBeNull();
  expect(screen.queryByRole("list", { name: "Launch work panes" })).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "Release notes" }));
  fireEvent.contextMenu(screen.getByRole("button", { name: "Release notes" }));
  expect(screen.getByLabelText("Rename Release notes")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove Release notes from Workspaces" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "xyzzy" } });
  expect(screen.getByText("No workspaces match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Launch work" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Release notes" })).toBeNull();
});
