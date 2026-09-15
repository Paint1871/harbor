// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { ThreadRecord } from "@harbor/schema/commands";
import { ThreadList, threadVisible } from "./ThreadList";

afterEach(cleanup);

const launch: ThreadRecord = { id: "a", workspaceId: "project", engineId: "opencode", title: "Launch work", pinned: false, unread: false };
const notes: ThreadRecord = { id: "b", workspaceId: "project", engineId: "claude-code", title: "Release notes", pinned: false, unread: false };

function renderList(threads: ThreadRecord[] = [launch, notes]) {
  render(
    <ThreadList
      threads={threads}
      activeId={null}
      onSelect={() => undefined}
      onPin={() => undefined}
      onRename={() => undefined}
      onDelete={() => undefined}
    />,
  );
}

const followUp = { title: "Inbox follow-up", engineId: "opencode", unread: true };
const launchMeta = { title: launch.title, engineId: launch.engineId, unread: launch.unread };

it("treats a blank query as a match unless unread-only hides a read thread", () => {
  expect(threadVisible(launchMeta, "", false)).toBe(true);
  expect(threadVisible(launchMeta, "   ", false)).toBe(true);
  expect(threadVisible(followUp, "", false)).toBe(true);
  expect(threadVisible(launchMeta, "", true)).toBe(false);
  expect(threadVisible(followUp, "", true)).toBe(true);
});

it("matches title and engine id case-insensitively after the unread gate", () => {
  expect(threadVisible(launchMeta, "LAUNCH", false)).toBe(true);
  expect(threadVisible(launchMeta, "OpenCode", false)).toBe(true);
  expect(threadVisible(followUp, "inbox", true)).toBe(true);
  expect(threadVisible(followUp, "OPENCODE", true)).toBe(true);
  expect(threadVisible(launchMeta, "launch", true)).toBe(false);
  expect(threadVisible(followUp, "xyzzy", true)).toBe(false);
  expect(threadVisible(launchMeta, "xyzzy", false)).toBe(false);
});

it("filters the rail by title and restores every row when the query is cleared", () => {
  renderList();
  expect(screen.getByTitle("Launch work")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search threads" }), { target: { value: "launch" } });
  expect(screen.getByTitle("Launch work")).toBeTruthy();
  expect(screen.queryByTitle("Release notes")).toBeNull();
  expect(screen.getByRole("button", { name: "Pin Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Rename Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Delete Launch work" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search threads" }), { target: { value: "" } });
  expect(screen.getByTitle("Launch work")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();
});

it("matches an engine id and shows empty-match copy when nothing remains", () => {
  renderList();
  const field = screen.getByRole("textbox", { name: "Search threads" });

  fireEvent.change(field, { target: { value: "claude" } });
  expect(screen.getByTitle("Release notes")).toBeTruthy();
  expect(screen.queryByTitle("Launch work")).toBeNull();

  fireEvent.change(field, { target: { value: "xyzzy" } });
  expect(screen.getByText("No threads match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByTitle("Launch work")).toBeNull();
  expect(screen.queryByTitle("Release notes")).toBeNull();
});

it("hides a read thread when Unread is pressed and restores it when turned off", () => {
  renderList([launch, { ...notes, unread: true }]);
  expect(screen.getByTitle("Launch work")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Unread" }));
  expect(screen.getByRole("button", { name: "Unread", pressed: true })).toBeTruthy();
  expect(screen.queryByTitle("Launch work")).toBeNull();
  expect(screen.getByTitle("Release notes")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Pin Release notes" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Rename Release notes" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Delete Release notes" })).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Unread", pressed: true }));
  expect(screen.getByRole("button", { name: "Unread", pressed: false })).toBeTruthy();
  expect(screen.getByTitle("Launch work")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();
});

it("shows unread empty copy, including when a search also matches nothing", () => {
  renderList();
  fireEvent.click(screen.getByRole("button", { name: "Unread" }));
  expect(screen.getByText("No unread threads.")).toBeTruthy();
  expect(screen.queryByTitle("Launch work")).toBeNull();
  expect(screen.queryByTitle("Release notes")).toBeNull();

  fireEvent.change(screen.getByRole("textbox", { name: "Search threads" }), { target: { value: "launch" } });
  expect(screen.getByText("No unread threads match “launch”.")).toBeTruthy();
});
