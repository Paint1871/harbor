// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ChromeProvider, type ChromeValue } from "../../chrome/chrome-context";
import { CodeMode } from "./CodeMode";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => undefined }));
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    loadAddon() {}
    open() {}
    writeln() {}
    write() {}
    dispose() {}
    onData() {
      return { dispose() {} };
    }
  },
}));
vi.mock("@xterm/addon-fit", () => ({ FitAddon: class { fit() {} } }));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

const chrome: ChromeValue = {
  mode: "code",
  theme: "black",
  profileName: "Local",
  destination: "mode",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
};

const workspace = { id: "ws-1", folder: "/tmp/project", title: "Project", pinned: false };
const tab = {
  id: "tab-1",
  workspaceId: "ws-1",
  layout: {
    type: "split" as const,
    dir: "h",
    ratio: 0.4,
    a: { type: "leaf" as const, paneId: "term-1" },
    b: { type: "leaf" as const, paneId: "files-1" },
  },
  panes: [
    { id: "term-1", kind: "terminal", paused: true },
    { id: "files-1", kind: "files", paused: false },
  ],
};

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "layout_restore") return Promise.resolve([tab]);
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "workspace_ensure_tab") return Promise.resolve(tab);
    if (command === "pane_create") return Promise.resolve("files-2");
    if (command === "workspace_save_layout") return Promise.resolve();
    if (command === "settings_set") return Promise.resolve();
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

it("restores a saved tab layout and keeps the terminal paused", async () => {
  render(
    <ChromeProvider value={chrome}>
      <CodeMode />
    </ChromeProvider>,
  );
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("layout_restore"));
  expect(await screen.findByText("Terminal paused")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Resume terminal" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Resize panes" })).toBeTruthy();
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("workspace_save_layout", {
      tabId: "tab-1",
      layout: tab.layout,
    }),
  );
});

it("adds a real files pane from the files pane header", async () => {
  render(
    <ChromeProvider value={chrome}>
      <CodeMode />
    </ChromeProvider>,
  );
  await screen.findByText("Terminal paused");
  fireEvent.click(screen.getByRole("button", { name: "Split Files" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("pane_create", {
      tabId: "tab-1",
      kind: "files",
      state: { kind: "files", cwd: "/tmp/project", paused: false },
    }),
  );
  expect(await screen.findByRole("button", { name: "Files pane 2" })).toBeTruthy();
});

it("lets a terminal pane choose a different CLI from its actions menu", async () => {
  render(
    <ChromeProvider value={chrome}>
      <CodeMode />
    </ChromeProvider>,
  );
  await screen.findByText("Terminal paused");
  fireEvent.click(screen.getByRole("button", { name: "More Claude Code" }));
  fireEvent.change(screen.getByRole("combobox", { name: "CLI for Claude Code" }), { target: { value: "shell" } });
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("pane_set_engine", { id: "term-1", engineId: "shell" }));
});
