// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AgentChat, AgentRecord, Memory, Place, SearchHit } from "@harbor/schema/commands";
import { AgentPage } from "./AgentPage";
import { GearPanel } from "./GearPanel";

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
vi.mock("./Face", async () => {
  const React = await import("react");
  return {
    Face: ({ name }: { name: string }) =>
      React.createElement("span", { className: "harbor-face" }, name),
  };
});

const agent: AgentRecord = {
  id: "agent-1",
  name: "Release manager",
  brief: "Ship the launch",
  engineId: "opencode",
  faceIndex: 0,
  pinned: false,
};

const created: AgentChat = {
  id: "chat-a",
  agentId: "agent-1",
  title: "New chat",
  status: "idle",
};

let chats: AgentChat[];
let memories: Memory[];
let places: Place[];
let transcripts: Record<string, { id: string; role: "user" | "assistant"; text: string }[]>;

beforeEach(() => {
  chats = [];
  memories = [];
  places = [];
  transcripts = {};
  mocks.handlers.clear();
  invoke.mockReset();
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "agent_chat_list") return Promise.resolve(chats);
    if (command === "agent_chat_history") return Promise.resolve(transcripts[String(args?.chatId)] ?? []);
    if (command === "agent_chat_create") {
      chats = [...chats, created];
      return Promise.resolve(created);
    }
    if (command === "agent_chat_send") {
      const parts = args?.parts as { text?: string }[] | undefined;
      const text = parts?.[0]?.text ?? "";
      chats = chats.map((chat) => (chat.id === args?.chatId ? { ...chat, title: text.slice(0, 64) } : chat));
      return Promise.resolve();
    }
    if (command === "agent_chat_cancel") return Promise.resolve();
    if (command === "memory_list") return Promise.resolve(memories);
    if (command === "memory_upsert") {
      const row: Memory = { id: "mem-1", body: String(args?.body ?? ""), kind: "fact" };
      memories = [...memories, row];
      return Promise.resolve(row);
    }
    if (command === "memory_delete") {
      memories = memories.filter((item) => item.id !== args?.id);
      return Promise.resolve();
    }
    if (command === "places_list") return Promise.resolve(places);
    if (command === "places_grant") {
      places = [...places, { id: "place-new", path: String(args?.path ?? "") }];
      return Promise.resolve();
    }
    if (command === "places_revoke") {
      places = places.filter((item) => item.id !== args?.id);
      return Promise.resolve();
    }
    if (command === "agent_update") return Promise.resolve();
    if (command === "face_preview") return Promise.resolve("data:image/svg+xml,face");
    if (command === "workspace_pick_folder") return Promise.resolve("/tmp/project");
    if (command === "acp_permission_resolve") return Promise.resolve();
    if (command === "session_search") return Promise.resolve([]);
    if (command === "plugin_list") return Promise.resolve([{
      id: "github",
      displayName: "GitHub",
      status: "connected",
      accountLabel: "Test account",
      description: "Repositories, issues, and pull requests",
      category: "Development",
      authKind: "device",
    }]);
    if (command === "plugin_grants_list") return Promise.resolve([]);
    if (command === "plugin_set_agent_grant") return Promise.resolve();
    if (command === "agent_list") return Promise.resolve([agent]);
    if (command === "mail_send") return Promise.resolve();
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

it("loads host chats instead of a fake chat-1 tab", async () => {
  chats = [{ id: "weekly", agentId: "agent-1", title: "Weekly notes", status: "idle" }];
  render(<AgentPage agent={agent} />);
  expect(await screen.findByRole("button", { name: "Weekly notes" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "chat-1" })).toBeNull();
  expect(invoke).toHaveBeenCalledWith("agent_chat_list", { agentId: "agent-1" });
});

it("loads persisted history when a chat is selected", async () => {
  chats = [
    { id: "weekly", agentId: "agent-1", title: "Weekly notes", status: "idle" },
    { id: "other", agentId: "agent-1", title: "Other", status: "idle" },
  ];
  transcripts.other = [{ id: "m1", role: "user", text: "Persisted line" }];
  render(<AgentPage agent={agent} />);
  fireEvent.click(await screen.findByRole("button", { name: "Other" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("agent_chat_history", { chatId: "other" }));
  expect(await screen.findByText("Persisted line", { selector: ".harbor-bubble-user" })).toBeTruthy();
});

it("creates a chat and sends text parts through invoke", async () => {
  render(<AgentPage agent={agent} />);
  fireEvent.click(await screen.findByRole("button", { name: "New chat" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("agent_chat_create", { agentId: "agent-1" }));
  const composer = await screen.findByRole("textbox", { name: "Message" });
  fireEvent.change(composer, { target: { value: "Ship the checklist" } });
  fireEvent.click(screen.getByRole("button", { name: "Send" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("agent_chat_send", {
      chatId: "chat-a",
      parts: [{ type: "text", text: "Ship the checklist" }],
    }),
  );
  expect(screen.getByText("Ship the checklist", { selector: ".harbor-bubble-user" })).toBeTruthy();
  expect(await screen.findByText(/Waiting on engine/)).toBeTruthy();
});

it("resolves a permission card for the active chat", async () => {
  chats = [created];
  render(<AgentPage agent={agent} />);
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("agent_chat_history", { chatId: "chat-a" }));
  await waitFor(() => expect(mocks.handlers.get("acp_permission")?.size).toBeTruthy());
  emitLocal("acp_permission", {
    id: "perm-1",
    sessionRef: "chat-a",
    title: "Run a command",
    command: "ls",
    options: [{ optionId: "opt-allow", kind: "allow_once", name: "Allow" }],
  });
  fireEvent.click(await screen.findByRole("button", { name: "Allow" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("acp_permission_resolve", {
      id: "perm-1",
      optionId: "opt-allow",
      cancelled: false,
    }),
  );
});

it("lists memory and upserts a fact", async () => {
  memories = [{ id: "existing", body: "Prefers tests", kind: "fact" }];
  render(<GearPanel agent={agent} />);
  expect(await screen.findByText(/Prefers tests/)).toBeTruthy();
  fireEvent.change(screen.getByPlaceholderText("Prefers tests before merge"), { target: { value: "No secrets" } });
  fireEvent.click(screen.getByRole("button", { name: "Remember" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("memory_upsert", { agentId: "agent-1", body: "No secrets" }),
  );
  expect(await screen.findByText(/No secrets/)).toBeTruthy();
});

it("lists places on open and revokes by id", async () => {
  places = [{ id: "place-1", path: "/tmp/project" }];
  render(<GearPanel agent={agent} />);
  expect(await screen.findByText("/tmp/project")).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("places_list", { agentId: "agent-1" });
  fireEvent.click(screen.getByRole("button", { name: "Revoke" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("places_revoke", { id: "place-1" }));
  await waitFor(() => expect(screen.queryByText("/tmp/project")).toBeNull());
});

it("searches past chats and can open a hit", async () => {
  const hits: SearchHit[] = [{ chat_id: "weekly", prose: "alpha rust backend", created_at: Math.floor(Date.now() / 1000) }];
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "session_search") return Promise.resolve(hits);
    if (command === "memory_list" || command === "places_list" || command === "plugin_grants_list") return Promise.resolve([]);
    return Promise.resolve([]);
  });
  const onOpenChat = vi.fn();
  render(<GearPanel agent={agent} onOpenChat={onOpenChat} />);
  fireEvent.change(await screen.findByLabelText("Search chats"), { target: { value: "rust" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("session_search", { agentId: "agent-1", query: "rust" }),
  );
  fireEvent.click(await screen.findByRole("button", { name: /alpha rust backend/ }));
  expect(onOpenChat).toHaveBeenCalledWith("weekly");
});

it("toggles the GitHub plugin grant for the current agent", async () => {
  render(<GearPanel agent={agent} />);
  const box = await screen.findByRole("checkbox", { name: "GitHub" });
  fireEvent.click(box);
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("plugin_set_agent_grant", {
      agentId: "agent-1",
      pluginId: "github",
      enabled: true,
    }),
  );
});

it("lists teammates from @ and sends mail", async () => {
  const other: AgentRecord = { ...agent, id: "agent-2", name: "Reviewer" };
  invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "agent_list") return Promise.resolve([agent, other]);
    if (command === "agent_chat_list") return Promise.resolve(chats);
    if (command === "agent_chat_history") return Promise.resolve([]);
    if (command === "mail_send") return Promise.resolve();
    return Promise.resolve([]);
  });
  render(<AgentPage agent={agent} />);
  const composer = await screen.findByRole("textbox", { name: "Message" });
  fireEvent.change(composer, { target: { value: "@" } });
  expect(await screen.findByRole("button", { name: "Reviewer" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Release manager" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Reviewer" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("mail_send", {
      fromAgentId: "agent-1",
      toAgentId: "agent-2",
      body: "Handoff from Release manager",
    }),
  );
  expect(screen.queryByRole("button", { name: agent.name })).toBeNull();
});
