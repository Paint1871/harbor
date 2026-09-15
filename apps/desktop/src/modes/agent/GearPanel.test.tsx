// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AgentRecord, Memory } from "@harbor/schema/commands";
import { GearPanel } from "./GearPanel";
import { memoryMatchesQuery } from "./memoryMatchesQuery";

const mocks = vi.hoisted(() => ({ call: vi.fn() }));

vi.mock("../../ipc", () => ({ call: mocks.call }));
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

const facts: Memory[] = [
  { id: "mem-tests", body: "Prefers tests before merge", kind: "fact" },
  { id: "mem-commits", body: "Uses conventional commits", kind: "fact" },
];

let memories: Memory[];

beforeEach(() => {
  memories = facts.map((item) => ({ ...item }));
  mocks.call.mockReset();
  mocks.call.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === "memory_list") return Promise.resolve(memories);
    if (command === "memory_upsert") {
      const row: Memory = { id: `mem-${memories.length + 1}`, body: String(args?.body ?? ""), kind: "fact" };
      memories = [...memories, row];
      return Promise.resolve(row);
    }
    if (command === "memory_delete") {
      memories = memories.filter((item) => item.id !== args?.id);
      return Promise.resolve();
    }
    return Promise.resolve([]);
  });
});
afterEach(cleanup);

function searchMemory() {
  return screen.getByRole("textbox", { name: "Search memory" });
}

function expectFactsFor(query: string) {
  for (const item of memories) {
    if (memoryMatchesQuery(item.body, query)) {
      expect(screen.getByText(item.body)).toBeTruthy();
    } else {
      expect(screen.queryByText(item.body)).toBeNull();
    }
  }
}

it("filters remembered facts as you type and restores every row when the query is cleared", async () => {
  render(<GearPanel agent={agent} />);
  expect(await screen.findByText("Prefers tests before merge")).toBeTruthy();
  expect(screen.getByText("Uses conventional commits")).toBeTruthy();
  expect(screen.getByPlaceholderText("Find a fact")).toBeTruthy();

  fireEvent.change(searchMemory(), { target: { value: "TESTS" } });
  expectFactsFor("TESTS");
  expect(screen.getByRole("button", { name: "Remove" })).toBeTruthy();
  expect(screen.getByPlaceholderText("Prefers tests before merge")).toBeTruthy();

  fireEvent.change(searchMemory(), { target: { value: "" } });
  expectFactsFor("");
});

it("shows empty-match copy and still lets a fact be added while filtering", async () => {
  render(<GearPanel agent={agent} />);
  await screen.findByText("Prefers tests before merge");

  fireEvent.change(searchMemory(), { target: { value: "xyzzy" } });
  expect(screen.getByText("No facts match “xyzzy”.")).toBeTruthy();
  expectFactsFor("xyzzy");
  expect(screen.getByPlaceholderText("Prefers tests before merge")).toBeTruthy();

  fireEvent.change(searchMemory(), { target: { value: "tests" } });
  expectFactsFor("tests");
  fireEvent.change(screen.getByPlaceholderText("Prefers tests before merge"), { target: { value: "Also prefers tests" } });
  fireEvent.click(screen.getByRole("button", { name: "Remember" }));
  await waitFor(() =>
    expect(mocks.call).toHaveBeenCalledWith("memory_upsert", { agentId: "agent-1", body: "Also prefers tests" }),
  );
  expect(await screen.findByText("Also prefers tests")).toBeTruthy();
  expectFactsFor("tests");
});

it("removes a visible fact while a filter is active", async () => {
  render(<GearPanel agent={agent} />);
  await screen.findByText("Prefers tests before merge");

  fireEvent.change(searchMemory(), { target: { value: "commit" } });
  expectFactsFor("commit");
  fireEvent.click(screen.getByRole("button", { name: "Remove" }));
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("memory_delete", { id: "mem-commits" }));
  await waitFor(() => expect(screen.queryByText("Uses conventional commits")).toBeNull());
  expect(screen.getByText("No facts match “commit”.")).toBeTruthy();
  expect(screen.queryByText("Prefers tests before merge")).toBeNull();
});

it("asks before deleting a teammate, then removes it through the host", async () => {
  const onDeleted = vi.fn();
  render(<GearPanel agent={agent} onDeleted={onDeleted} />);
  const remove = await screen.findByRole("button", { name: "Delete Release manager" });
  expect(remove.textContent).toBe("Delete teammate");

  fireEvent.click(remove);
  expect(mocks.call).not.toHaveBeenCalledWith("agent_delete", expect.anything());
  expect(remove.textContent).toBe("Delete?");

  fireEvent.click(remove);
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("agent_delete", { id: "agent-1" }));
  await waitFor(() => expect(onDeleted).toHaveBeenCalledWith("agent-1"));
});

it("keeps the teammate and shows the gear error when delete fails", async () => {
  const onDeleted = vi.fn();
  mocks.call.mockImplementation((command: string) => {
    if (command === "agent_delete") return Promise.reject(new Error("nope"));
    if (command === "memory_list") return Promise.resolve(memories);
    return Promise.resolve([]);
  });
  render(<GearPanel agent={agent} onDeleted={onDeleted} />);
  const remove = await screen.findByRole("button", { name: "Delete Release manager" });
  fireEvent.click(remove);
  fireEvent.click(remove);
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toMatch(/could not be deleted/);
  expect(onDeleted).not.toHaveBeenCalled();
});
