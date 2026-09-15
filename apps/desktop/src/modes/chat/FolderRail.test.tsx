// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { Workspace } from "@harbor/schema/commands";
import { FolderRail } from "./FolderRail";

afterEach(cleanup);

const launch: Workspace = { id: "a", folder: "/tmp/launch", title: "Launch app", pinned: false };
const notes: Workspace = { id: "b", folder: "/tmp/notes", title: "Release notes", pinned: false };

function renderRail(otherCount = 2) {
  render(
    <FolderRail
      workspaces={[launch, notes]}
      otherCount={otherCount}
      selectedId={null}
      onAddWorkspace={() => undefined}
      onSelect={() => undefined}
    >
      <p>Threads go here</p>
    </FolderRail>,
  );
}

it("filters the rail by title and restores every row when the query is cleared", () => {
  renderRail();
  expect(screen.getByTitle("Launch app")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();
  expect(screen.getByTitle("Other chats")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add folder" })).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search folders" }), { target: { value: "launch" } });
  expect(screen.getByTitle("Launch app")).toBeTruthy();
  expect(screen.queryByTitle("Release notes")).toBeNull();
  expect(screen.getByTitle("Other chats")).toBeTruthy();

  fireEvent.change(screen.getByRole("textbox", { name: "Search folders" }), { target: { value: "" } });
  expect(screen.getByTitle("Launch app")).toBeTruthy();
  expect(screen.getByTitle("Release notes")).toBeTruthy();
  expect(screen.getByTitle("Other chats")).toBeTruthy();
});

it("matches a folder path and shows empty-match copy when nothing remains", () => {
  renderRail();
  const field = screen.getByRole("textbox", { name: "Search folders" });

  fireEvent.change(field, { target: { value: "notes" } });
  expect(screen.getByTitle("Release notes")).toBeTruthy();
  expect(screen.queryByTitle("Launch app")).toBeNull();
  expect(screen.getByTitle("Other chats")).toBeTruthy();

  fireEvent.change(field, { target: { value: "xyzzy" } });
  expect(screen.getByText("No folders match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByTitle("Launch app")).toBeNull();
  expect(screen.queryByTitle("Release notes")).toBeNull();
  expect(screen.getByTitle("Other chats")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add folder" })).toBeTruthy();
});
