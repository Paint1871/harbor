// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { AgentRecord } from "@harbor/schema/commands";
import { ChromeProvider, type ChromeValue } from "../../chrome/chrome-context";
import { AgentRail } from "./AgentRail";

vi.mock("./Face", async () => {
  const React = await import("react");
  return {
    Face: ({ name }: { name: string }) =>
      React.createElement("span", { className: "harbor-face" }, name),
  };
});

afterEach(cleanup);

const launch: AgentRecord = {
  id: "a",
  name: "Launch bot",
  brief: "Ship the launch",
  engineId: "opencode",
  faceIndex: 0,
  pinned: false,
};
const notes: AgentRecord = {
  id: "b",
  name: "Notes",
  brief: "Release notes",
  engineId: "claude-code",
  faceIndex: 1,
  pinned: false,
};

const chrome: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Ada",
  destination: "mode",
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

function renderRail(agents: AgentRecord[] = [launch, notes]) {
  render(
    <ChromeProvider value={chrome}>
      <AgentRail
        agents={agents}
        selectedId={null}
        onSelect={() => undefined}
        onNew={() => undefined}
        onPin={() => undefined}
      />
    </ChromeProvider>,
  );
}

it("filters the rail by name and restores every row when the query is cleared", () => {
  renderRail();
  expect(screen.getByTitle("Launch bot")).toBeTruthy();
  expect(screen.getByTitle("Notes")).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search agents" }), { target: { value: "launch" } });
  expect(screen.getByTitle("Launch bot")).toBeTruthy();
  expect(screen.queryByTitle("Notes")).toBeNull();
  expect(screen.getByRole("button", { name: "Pin Launch bot" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search agents" }), { target: { value: "" } });
  expect(screen.getByTitle("Launch bot")).toBeTruthy();
  expect(screen.getByTitle("Notes")).toBeTruthy();
});

it("matches a brief and shows empty-match copy when nothing remains", () => {
  renderRail();
  const field = screen.getByRole("textbox", { name: "Search agents" });

  fireEvent.change(field, { target: { value: "release" } });
  expect(screen.getByTitle("Notes")).toBeTruthy();
  expect(screen.queryByTitle("Launch bot")).toBeNull();

  fireEvent.change(field, { target: { value: "xyzzy" } });
  expect(screen.getByText("No agents match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByTitle("Launch bot")).toBeNull();
  expect(screen.queryByTitle("Notes")).toBeNull();
});
