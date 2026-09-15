// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { Workspace } from "@harbor/schema/commands";
import { WorkspaceRailList } from "./WorkspaceRailList";

afterEach(cleanup);

const launch: Workspace = { id: "a", folder: "/tmp/launch", title: "Launch work", pinned: false };
const notes: Workspace = { id: "b", folder: "/Users/x/notes-repo", title: "Release notes", pinned: false };

function renderList(workspaces: Workspace[] = [launch, notes]) {
  render(
    <WorkspaceRailList workspaces={workspaces} empty={<p>Add a folder to start coding.</p>}>
      {(workspace) => (
        <button type="button">{workspace.title}</button>
      )}
    </WorkspaceRailList>,
  );
}

it("filters the rail by title and restores every row when the query is cleared", () => {
  renderList();
  expect(screen.getByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "launch" } });
  expect(screen.getByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Release notes" })).toBeNull();

  fireEvent.change(screen.getByRole("textbox", { name: "Search workspaces" }), { target: { value: "" } });
  expect(screen.getByRole("button", { name: "Launch work" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();
});

it("matches a folder path and shows empty-match copy when nothing remains", () => {
  renderList();
  const field = screen.getByRole("textbox", { name: "Search workspaces" });

  fireEvent.change(field, { target: { value: "notes-repo" } });
  expect(screen.getByRole("button", { name: "Release notes" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Launch work" })).toBeNull();

  fireEvent.change(field, { target: { value: "xyzzy" } });
  expect(screen.getByText("No workspaces match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Launch work" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Release notes" })).toBeNull();
});

it("keeps the empty-rail copy when there is nothing to search", () => {
  renderList([]);
  expect(screen.getByRole("textbox", { name: "Search workspaces" })).toBeTruthy();
  expect(screen.getByPlaceholderText("Find a workspace")).toBeTruthy();
  expect(screen.getByText("Add a folder to start coding.")).toBeTruthy();
});
