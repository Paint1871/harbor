// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AddWorkspace } from "./AddWorkspace";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  settingsGet: vi.fn(),
  settingsSet: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../settings", () => ({
  settingsGet: mocks.settingsGet,
  settingsSet: mocks.settingsSet,
}));

const workspace = {
  id: "ws-setup",
  folder: "/tmp/harbor-project",
  title: "harbor-project",
  pinned: false,
};

beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value() {
      this.open = true;
    },
  });
  mocks.invoke.mockReset();
  mocks.settingsGet.mockResolvedValue(null);
  mocks.settingsSet.mockResolvedValue(undefined);
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "engines_detect") {
      return Promise.resolve([{ id: "claude-code", displayName: "Claude Code", path: "/usr/local/bin/claude", status: "ready", supportsChat: true, supportsTerminal: true }]);
    }
    if (command === "workspace_add") return Promise.resolve(workspace);
    if (command === "workspace_configure_tab") return Promise.resolve();
    return Promise.resolve(null);
  });
});

afterEach(cleanup);

it("sends the launch profile and remembers it for the next workspace", async () => {
  const onAdded = vi.fn();
  render(<AddWorkspace onAdded={onAdded} onClose={vi.fn()} />);
  await screen.findByText("1 installed CLI detected on this Mac.");

  fireEvent.change(screen.getByRole("textbox", { name: "Or enter a full folder path" }), {
    target: { value: workspace.folder },
  });
  fireEvent.change(screen.getByRole("slider", { name: "Additional terminals" }), {
    target: { value: "3" },
  });
  fireEvent.click(screen.getByRole("checkbox", { name: /Browser preview/ }));
  fireEvent.click(screen.getByRole("button", { name: "Open folder" }));

  await waitFor(() => expect(onAdded).toHaveBeenCalledWith(workspace));
  expect(mocks.invoke).toHaveBeenCalledWith("workspace_configure_tab", {
    workspaceId: workspace.id,
    setup: {
      additionalTerminals: 3,
      browserPreview: false,
      threadPane: true,
      terminalEngineIds: ["claude-code", "claude-code", "claude-code", "claude-code"],
    },
  });
  expect(mocks.settingsSet).toHaveBeenCalledWith("workspace_launch_defaults", {
    additionalTerminals: 3,
    browserPreview: false,
    threadPane: true,
    terminalEngineIds: ["claude-code", "claude-code", "claude-code", "claude-code"],
  });
});

it("keeps a different CLI choice for each terminal", async () => {
  const onAdded = vi.fn();
  render(<AddWorkspace onAdded={onAdded} onClose={vi.fn()} />);
  await screen.findByText("1 installed CLI detected on this Mac.");

  fireEvent.change(screen.getByRole("combobox", { name: "Terminal 1 CLI" }), {
    target: { value: "shell" },
  });
  fireEvent.change(screen.getByRole("combobox", { name: "Terminal 2 CLI" }), {
    target: { value: "claude-code" },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Or enter a full folder path" }), {
    target: { value: workspace.folder },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open folder" }));

  await waitFor(() => expect(onAdded).toHaveBeenCalledWith(workspace));
  expect(mocks.invoke).toHaveBeenCalledWith("workspace_configure_tab", {
    workspaceId: workspace.id,
    setup: {
      additionalTerminals: 1,
      browserPreview: true,
      threadPane: true,
      terminalEngineIds: ["shell", "claude-code"],
    },
  });
});
