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
    if (command === "engines_detect") return Promise.resolve([
      { id: "opencode", displayName: "OpenCode", status: "ready", supportsChat: true },
      { id: "claude-code", displayName: "Claude Code", status: "ready", supportsChat: true },
    ]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve(stored);
    if (command === "thread_send") { stored = [{ id: "saved", role: "user", text: "Keep my message" }]; return Promise.reject(new Error("engine offline")); }
    if (command === "thread_set_config") return Promise.resolve();
    if (command === "thread_set_engine") return Promise.resolve();
    if (command === "thread_config_options") return Promise.resolve([{
      id: "model",
      name: "Model",
      category: "model",
      currentValue: "opencode/big-pickle",
      values: [
        { value: "opencode/big-pickle", name: "OpenCode Zen/Big Pickle" },
        { value: "forge/kimi-k3", name: "Forge AI/Kimi K3" },
      ],
    }]);
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
  fireEvent.click(screen.getByRole("button", { name: "Browse…" }));
  const path = screen.getByRole("textbox", { name: "Folder path" }) as HTMLInputElement;
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

it("switches the thread to another engine from the composer", async () => {
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  await screen.findByRole("textbox", { name: "Message" });

  fireEvent.click(screen.getByRole("button", { name: /OpenCode/ }));
  fireEvent.click(screen.getByRole("menuitemradio", { name: /Claude Code/ }));

  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_set_engine", { id: thread.id, engineId: "claude-code" }));
  expect(await screen.findByRole("button", { name: "Engine: Claude Code" })).toBeTruthy();
});

it("reports the conversation size without claiming a context window", async () => {
  stored = [
    { id: "a", role: "user", text: "x".repeat(4000) },
    { id: "b", role: "assistant", text: "y".repeat(4000) },
  ];
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));

  const meter = await screen.findByText("~2.0k tokens · 2 messages");
  expect(meter.getAttribute("title")).toMatch(/four characters per token/);
  expect(screen.queryByText(/%/)).toBeNull();
});

it("shows the engine's model as its own control, grouped by provider", async () => {
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));

  // Opening the thread is enough; the model is not hidden behind the engine menu.
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_config_options", { id: thread.id }));
  fireEvent.click(await screen.findByRole("button", { name: "Model: Big Pickle" }));

  // The provider half of a qualified name becomes the group heading.
  expect(screen.getByText("OpenCode Zen")).toBeTruthy();
  expect(screen.getByText("Forge AI")).toBeTruthy();
  expect(screen.getByRole("menuitemradio", { name: /Big Pickle/ }).getAttribute("aria-checked")).toBe("true");

  fireEvent.click(screen.getByRole("menuitemradio", { name: /Kimi K3/ }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_set_config", {
    id: thread.id,
    optionId: "model",
    value: "forge/kimi-k3",
  }));
  expect(await screen.findByRole("button", { name: "Model: Kimi K3" })).toBeTruthy();
});

it("frees the composer as soon as the message is in the transcript", async () => {
  let release: () => void = () => {};
  const turn = new Promise<void>((resolve) => { release = resolve; });
  const base = invoke.getMockImplementation()!;
  invoke.mockImplementation((command: string, args: unknown) => command === "thread_send" ? turn : base(command, args));

  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  const composer = await screen.findByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
  await waitFor(() => expect(composer.hasAttribute("disabled")).toBe(false));
  fireEvent.change(composer, { target: { value: "Take your time" } });
  fireEvent.click(screen.getByRole("button", { name: "Send" }));

  // The turn is still running; the box is already the builder's again.
  await waitFor(() => expect(composer.value).toBe(""));
  expect(screen.getByText("Take your time", { selector: ".harbor-bubble-user" })).toBeTruthy();
  release();
});

it("tells the host which conversation is on screen", async () => {
  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("session_watch", { sessionRef: null }));
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("session_watch", { sessionRef: thread.id }));
});

it("does not let a text suggestion write the sent message back into the box", async () => {
  let release: () => void = () => {};
  const turn = new Promise<void>((resolve) => { release = resolve; });
  const base = invoke.getMockImplementation()!;
  invoke.mockImplementation((command: string, args: unknown) => command === "thread_send" ? turn : base(command, args));

  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  const composer = await screen.findByRole("textbox", { name: "Message" }) as HTMLTextAreaElement;
  await waitFor(() => expect(composer.hasAttribute("disabled")).toBe(false));

  // The platform is asked not to correct at all…
  expect(composer.getAttribute("autocorrect")).toBe("off");
  expect(composer.getAttribute("autocapitalize")).toBe("off");

  fireEvent.change(composer, { target: { value: "test" } });
  fireEvent.click(screen.getByRole("button", { name: "Send" }));
  await waitFor(() => expect(composer.value).toBe(""));

  // …and if it corrects anyway, the write-back is refused.
  fireEvent.change(composer, { target: { value: "test" } });
  await waitFor(() => expect(composer.value).toBe(""));

  // Typing it again is not a write-back, and still works.
  fireEvent.change(composer, { target: { value: "t" } });
  fireEvent.change(composer, { target: { value: "test" } });
  await waitFor(() => expect(composer.value).toBe("test"));
  release();
});

it("offers to add an engine whose ACP adapter is missing, then uses it", async () => {
  const base = invoke.getMockImplementation()!;
  const withAdapter = { id: "claude-code", displayName: "Claude Code", path: "/usr/bin/claude", status: "ready", supportsChat: true };
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "engines_detect") return Promise.resolve([
      { id: "opencode", displayName: "OpenCode", path: "/usr/bin/opencode", status: "ready", supportsChat: true },
      { id: "claude-code", displayName: "Claude Code", path: "/usr/bin/claude", status: "adapter-missing", supportsChat: false, adapterPackage: "@agentclientprotocol/claude-agent-acp@^0.75" },
    ]);
    if (command === "engine_install_adapter") return Promise.resolve([
      { id: "opencode", displayName: "OpenCode", path: "/usr/bin/opencode", status: "ready", supportsChat: true },
      withAdapter,
    ]);
    return base(command, args);
  });

  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  await screen.findByRole("textbox", { name: "Message" });
  fireEvent.click(screen.getByRole("button", { name: /OpenCode/ }));

  // Not selectable yet, but no longer invisible.
  expect(screen.queryByRole("menuitemradio", { name: /Claude Code/ })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Add" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("engine_install_adapter", { engineId: "claude-code" }));

  fireEvent.click(await screen.findByRole("menuitemradio", { name: /Claude Code/ }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("thread_set_engine", { id: thread.id, engineId: "claude-code" }));
});

it("filters a long model list instead of making you scroll it", async () => {
  const base = invoke.getMockImplementation()!;
  const many = Array.from({ length: 24 }, (_, index) => ({
    value: `vendor-${index}/model-${index}`,
    name: `Vendor ${index}/Model ${index}`,
  }));
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "thread_config_options") return Promise.resolve([
      { id: "model", name: "Model", category: "model", currentValue: many[0]!.value, values: many },
    ]);
    return base(command, args);
  });

  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  fireEvent.click(await screen.findByRole("button", { name: "Model: Model 0" }));

  expect(screen.getAllByRole("menuitemradio")).toHaveLength(24);
  fireEvent.change(screen.getByRole("textbox", { name: "Filter Model" }), { target: { value: "Model 17" } });
  const left = screen.getAllByRole("menuitemradio");
  expect(left).toHaveLength(1);
  expect(left[0]!.textContent).toContain("Model 17");
});

it("drops the old engine's models when the thread switches engine", async () => {
  const base = invoke.getMockImplementation()!;
  const byEngine: Record<string, unknown[]> = {
    opencode: [{ id: "model", name: "Model", category: "model", currentValue: "opencode/big-pickle", values: [{ value: "opencode/big-pickle", name: "OpenCode Zen/Big Pickle" }] }],
    "claude-code": [{ id: "model", name: "Model", category: "model", currentValue: "opus", values: [{ value: "opus", name: "Opus" }, { value: "sonnet", name: "Sonnet" }] }],
  };
  let engine = "opencode";
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "thread_config_options") return Promise.resolve(byEngine[engine]);
    if (command === "thread_set_engine") { engine = String(args?.engineId); return Promise.resolve(); }
    return base(command, args);
  });

  render(<ChromeProvider value={chrome}><ChatMode /></ChromeProvider>);
  fireEvent.click(await screen.findByRole("button", { name: "Start a thread" }));
  await screen.findByRole("button", { name: "Model: Big Pickle" });

  fireEvent.click(screen.getByRole("button", { name: "Engine: OpenCode" }));
  fireEvent.click(screen.getByRole("menuitemradio", { name: /Claude Code/ }));

  // A Claude model list must never sit under an OpenCode pill, or the reverse.
  await screen.findByRole("button", { name: "Engine: Claude Code" });
  await screen.findByRole("button", { name: "Model: Opus" });
  expect(screen.queryByRole("button", { name: "Model: Big Pickle" })).toBeNull();
});
