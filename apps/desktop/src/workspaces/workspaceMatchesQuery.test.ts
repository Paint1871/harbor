import { expect, it } from "vitest";
import { workspaceMatchesQuery } from "./workspaceMatchesQuery";

const workspace = { title: "Launch work", folder: "/Users/x/free-project" };

it("treats an empty or blank query as a match", () => {
  expect(workspaceMatchesQuery(workspace, "")).toBe(true);
  expect(workspaceMatchesQuery(workspace, "   ")).toBe(true);
});

it("matches title and folder case-insensitively", () => {
  expect(workspaceMatchesQuery(workspace, "launch")).toBe(true);
  expect(workspaceMatchesQuery(workspace, "FREE-PROJECT")).toBe(true);
  expect(workspaceMatchesQuery(workspace, "Users/x")).toBe(true);
});

it("matches the displayed name when the title is empty", () => {
  expect(workspaceMatchesQuery({ title: null, folder: "/tmp/Notes Repo" }, "notes")).toBe(true);
});

it("rejects workspaces that do not contain the query", () => {
  expect(workspaceMatchesQuery(workspace, "xyzzy")).toBe(false);
  expect(workspaceMatchesQuery({ title: "Ada", folder: "/tmp/ada" }, "launch")).toBe(false);
});
