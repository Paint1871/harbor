// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ChromeProvider, type ChromeValue } from "../../chrome/chrome-context";
import { ChatMode } from "./ChatMode";

const mocks = vi.hoisted(() => {
  const handlers = new Map<string, Set<(event: { payload: unknown }) => void>>();
  return {
    invoke: vi.fn(),
    handlers,
    listen: vi.fn(async (event: string, handler: (event: { payload: unknown }) => void) => {
      let set = handlers.get(event);
      if (!set) {
        set = new Set();
        handlers.set(event, set);
      }
      set.add(handler);
      return () => { set.delete(handler); };
    }),
    emitLocal(event: string, payload: unknown) {
      handlers.get(event)?.forEach((handler) => handler({ payload }));
    },
  };
});
const { invoke, emitLocal } = mocks;
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
const chrome: ChromeValue = { mode: "chat", theme: "black", profileName: "Local", destination: "mode", setDestination: () => {}, onModeChange: () => {}, onThemeChange: () => {}, onSettings: () => {}, onTidy: () => {}, registerTidy: () => {} };
const workspace = { id: "project", folder: "/tmp/project", title: "Project", pinned: false };
const thread = { id: "thread", workspaceId: "project", engineId: "opencode", title: "New thread", pinned: false, unread: false };
let stored: { id: string; role: string; text: string }[];
beforeEach(() => {
  stored = [];
  mocks.handlers.clear();
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", { configurable: true, value() { this.open = true; } });
  invoke.mockReset();
  invoke.mockImplementation((command) => {
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "engines_detect") return Promise.resolve([{ id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true }]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve(stored);
    if (command === "thread_send") { stored = [{ id: "saved", role: "user", text: "Keep my message" }]; return Promise.reject(new Error("engine offline")); }
    if (command === "thread_set_config") return Promise.resolve();
    if (command === "thread_attach_files") return Promise.resolve();
    if (command === "thread_cancel") return Promise.resolve();
    if (command === "acp_permission_resolve") return Promise.resolve();
    if (command === "workspace_pick_folder") return Promise.resolve(null);
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

it("requires a thread before composing, preserves a failed draft, and shows the stored message", async () => {
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  expect(screen.queryByRole("textbox", { name: "Message" })).toBeNull();
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  const composer = await screen.findByRole("textbox", { name: "Message" });
  await waitFor(() => expect(composer.hasAttribute("disabled")).toBe(false));
  fireEvent.change(composer, { target: { value: "Keep my message" } });
  fireEvent.click(screen.getByRole("button", { name: "Send" }));
  await screen.findByText(/The engine could not finish this message/);
  expect((composer as HTMLTextAreaElement).value).toBe("Keep my message");
  expect(screen.getByText("Keep my message", { selector: ".harbor-bubble-user" })).toBeTruthy();
  expect(document.querySelector("button button")).toBeNull();
});

it("adds and selects the native-picked folder without creating a thread automatically", async () => {
  invoke.mockImplementation((command) => {
    if (command === "workspace_pick_folder") return Promise.resolve(workspace.folder);
    if (command === "workspace_add") return Promise.resolve(workspace);
    return Promise.resolve([]);
  });
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: /Open a project folder/ }));
  fireEvent.click(screen.getByRole("button", { name: "Choose folder…" }));
  const path = screen.getByRole("textbox", { name: "Or enter a full folder path" }) as HTMLInputElement;
  await waitFor(() => expect(path.value).toBe(workspace.folder));
  fireEvent.click(screen.getByRole("button", { name: "Open folder" }));
  await screen.findByText("Let’s work on Project.");
  expect(invoke).toHaveBeenCalledWith("workspace_add", { folder: workspace.folder });
  expect(invoke.mock.calls.some(([command]) => command === "thread_create")).toBe(false);
});

it("attaches a folder through thread_attach_files", async () => {
  invoke.mockImplementation((command) => {
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "engines_detect") return Promise.resolve([{ id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true }]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve([]);
    if (command === "workspace_pick_folder") return Promise.resolve("/tmp/notes");
    if (command === "thread_attach_files") return Promise.resolve();
    return Promise.resolve([]);
  });
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  fireEvent.click(await screen.findByRole("button", { name: "Attach folder" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_attach_files", { id: thread.id, paths: ["/tmp/notes"] }));
});

it("stops an in-flight turn with thread_cancel", async () => {
  let finishSend!: () => void;
  invoke.mockImplementation((command) => {
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "engines_detect") return Promise.resolve([{ id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true }]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve([]);
    if (command === "thread_send") return new Promise<void>((resolve) => { finishSend = resolve; });
    if (command === "thread_cancel") return Promise.resolve();
    return Promise.resolve([]);
  });
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  const composer = await screen.findByRole("textbox", { name: "Message" });
  await waitFor(() => expect(composer.hasAttribute("disabled")).toBe(false));
  fireEvent.change(composer, { target: { value: "Please stop me" } });
  fireEvent.click(screen.getByRole("button", { name: "Send" }));
  fireEvent.click(await screen.findByRole("button", { name: "Stop" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_cancel", { id: thread.id }));
  finishSend();
});

it("lists workspace files from @ and does not list teammates", async () => {
  invoke.mockImplementation((command) => {
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "engines_detect") return Promise.resolve([{ id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true }]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve([]);
    if (command === "fs_list") return Promise.resolve([{ name: "README.md", path: "/tmp/project/README.md", directory: false }]);
    if (command === "agent_list") return Promise.resolve([{ id: "agent-1", name: "Reviewer", brief: "", engineId: "opencode", faceIndex: 0, pinned: false }]);
    return Promise.resolve([]);
  });
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  const composer = await screen.findByRole("textbox", { name: "Message" });
  fireEvent.change(composer, { target: { value: "@" } });
  expect(await screen.findByRole("button", { name: "README.md" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Reviewer" })).toBeNull();
  expect(invoke.mock.calls.some(([command]) => command === "agent_list")).toBe(false);
  expect(invoke).toHaveBeenCalledWith("fs_list", { workspaceId: workspace.id, path: "" });
});

it("mounts a permission card and resolves with the option id", async () => {
  invoke.mockImplementation((command) => {
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "engines_detect") return Promise.resolve([{ id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true }]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve([]);
    if (command === "acp_permission_resolve") return Promise.resolve();
    return Promise.resolve([]);
  });
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_history", { id: thread.id }));
  await waitFor(() => expect(mocks.handlers.get("acp_permission")?.size).toBeTruthy());
  emitLocal("acp_permission", {
    id: "perm-chat",
    sessionRef: thread.id,
    title: "Read a file",
    path: "/tmp/project/README.md",
    options: [{ optionId: "opt-allow", kind: "allow_once", name: "Allow" }],
  });
  fireEvent.click(await screen.findByRole("button", { name: "Allow" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("acp_permission_resolve", {
      id: "perm-chat",
      optionId: "opt-allow",
      cancelled: false,
    }),
  );
});
