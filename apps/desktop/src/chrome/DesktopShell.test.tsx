// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { DesktopShell } from "./DesktopShell";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
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
vi.mock("../modes/agent/Face", async () => {
  const React = await import("react");
  return {
    Face: ({ name }: { name: string }) =>
      React.createElement("span", { className: "harbor-face" }, name),
  };
});

const agent = {
  id: "agent-1",
  name: "Release manager",
  brief: "Ship the launch",
  engineId: "opencode",
  faceIndex: 0,
  pinned: false,
};
const workspace = { id: "ws-1", folder: "/tmp/project", title: "Project", pinned: false };
const thread = { id: "thread-1", workspaceId: "ws-1", engineId: "opencode", title: "New thread", pinned: false, unread: false };
const chat = { id: "chat-1", agentId: "agent-1", title: "New chat", status: "idle" };
const engine = {
  id: "opencode",
  displayName: "OpenCode",
  status: "ready",
  path: "/usr/bin/opencode",
  supportsChat: true,
  supportsTerminal: true,
};

function press(key: string, init: KeyboardEventInit = {}) {
  fireEvent.keyDown(window, { key, bubbles: true, ...init });
}

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.listen.mockReset();
  mocks.listen.mockResolvedValue(() => undefined);
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "agent_list") return Promise.resolve([agent]);
    if (command === "agent_chat_list") return Promise.resolve([]);
    if (command === "agent_chat_create") return Promise.resolve(chat);
    if (command === "agent_chat_history") return Promise.resolve([]);
    if (command === "workspace_list") return Promise.resolve([workspace]);
    if (command === "thread_list") return Promise.resolve([]);
    if (command === "thread_create") return Promise.resolve(thread);
    if (command === "thread_history") return Promise.resolve([]);
    if (command === "engines_detect") return Promise.resolve([engine]);
    if (command === "layout_restore") return Promise.resolve([]);
    if (command === "notifications_list") return Promise.resolve([]);
    if (command === "notifications_unread_count") return Promise.resolve(0);
    if (command === "usage_bridge_status") {
      return Promise.resolve({ connected: false, chained: null, settingsPath: "" });
    }
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

it("opens Settings on Cmd+, and closes it on Escape", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="agent" />,
  );
  press(",", { code: "Comma", metaKey: true });
  expect(await screen.findByRole("dialog", { name: "Settings" })).toBeTruthy();
  expect(screen.getByRole("heading", { name: "General" })).toBeTruthy();
  press("Escape");
  await waitFor(() => expect(screen.queryByRole("dialog", { name: "Settings" })).toBeNull());
});

it("closes the Inbox on Escape", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="agent" />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Notifications" }));
  expect(document.querySelector(".harbor-inbox")?.getAttribute("data-open")).toBe("true");
  press("Escape");
  await waitFor(() => expect(document.querySelector(".harbor-inbox")?.getAttribute("data-open")).toBe("false"));
});

it("creates an agent chat on Cmd+N even with the composer focused", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="agent" />,
  );
  const composer = await screen.findByRole("textbox", { name: "Message" });
  composer.focus();
  fireEvent.keyDown(composer, { key: "n", code: "KeyN", metaKey: true, bubbles: true });
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("agent_chat_create", { agentId: "agent-1" }));
});

it("creates a Chat thread on Cmd+N in Chat mode", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="chat" />,
  );
  await screen.findByRole("button", { name: "Start a thread" });
  press("n", { code: "KeyN", metaKey: true });
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("thread_create", { workspaceId: "ws-1", engineId: "opencode" }),
  );
});

it("does not spawn a terminal on Cmd+N in Code mode", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="code" />,
  );
  await screen.findByRole("button", { name: /Project/ });
  press("n", { code: "KeyN", metaKey: true });
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("workspace_list"));
  expect(mocks.invoke.mock.calls.some(([command]) => command === "thread_create")).toBe(false);
  expect(mocks.invoke.mock.calls.some(([command]) => command === "pty_spawn")).toBe(false);
  expect(mocks.invoke.mock.calls.some(([command]) => command === "agent_chat_create")).toBe(false);
});

it("jumps to Chat and creates a thread in the Code folder on Opt+Cmd+T", async () => {
  render(
    <DesktopShell theme="black" onThemeChange={() => undefined} profileName="Ada" initialMode="code" />,
  );
  await screen.findByRole("button", { name: /Project/ });
  press("t", { code: "KeyT", metaKey: true, altKey: true });
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("thread_create", { workspaceId: "ws-1", engineId: "opencode" }),
  );
  expect(document.querySelector('.harbor-mode[aria-label="Chat"]')?.getAttribute("data-active")).toBe("true");
});
