// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { ThreadRecord } from "@harbor/schema/commands";
import { ThreadList } from "./ThreadList";

afterEach(cleanup);

const launch: ThreadRecord = { id: "a", workspaceId: "project", engineId: "opencode", title: "Launch work", pinned: false, unread: false };
const notes: ThreadRecord = { id: "b", workspaceId: "project", engineId: "claude-code", title: "Release notes", pinned: false, unread: false };

function renderList() {
  render(
    <ThreadList
      threads={[launch, notes]}
      activeId={null}
      onSelect={() => undefined}
      onPin={() => undefined}
      onRename={() => undefined}
      onDelete={() => undefined}
    />,
  );
}

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
