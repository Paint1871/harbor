import { expect, it } from "vitest";
import { folderMatchesQuery } from "./FolderRail";

const workspace = { title: "Launch app", folder: "/tmp/release-notes" };

it("treats an empty or blank query as a match", () => {
  expect(folderMatchesQuery(workspace, "")).toBe(true);
  expect(folderMatchesQuery(workspace, "   ")).toBe(true);
});

it("matches title and folder case-insensitively", () => {
  expect(folderMatchesQuery(workspace, "launch")).toBe(true);
  expect(folderMatchesQuery(workspace, "APP")).toBe(true);
  expect(folderMatchesQuery(workspace, "Release")).toBe(true);
  expect(folderMatchesQuery(workspace, "/TMP/")).toBe(true);
});

it("rejects folders that do not contain the query", () => {
  expect(folderMatchesQuery(workspace, "xyzzy")).toBe(false);
  expect(folderMatchesQuery({ title: null, folder: "/tmp/notes" }, "launch")).toBe(false);
  expect(folderMatchesQuery({ folder: "/tmp/notes" }, "launch")).toBe(false);
});
