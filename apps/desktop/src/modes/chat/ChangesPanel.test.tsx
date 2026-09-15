// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ChangesPanel, changesSummaryLabel } from "./ChangesPanel";

const mocks = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../../ipc", () => ({ call: mocks.call }));

const diffs = [
  { path: "src/a.rs", patch: "diff --git a/src/a.rs b/src/a.rs\n+one\n" },
  { path: "notes.md", patch: "untracked\nnew file: notes.md\n" },
  { path: "README.md", patch: "untracked\nnew file: README.md\n" },
];

beforeEach(() => {
  mocks.call.mockReset();
  mocks.call.mockResolvedValue(diffs);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("labels the summary as Changes, then Changes · N", () => {
  expect(changesSummaryLabel(0)).toBe("Changes");
  expect(changesSummaryLabel(1)).toBe("Changes · 1");
  expect(changesSummaryLabel(3)).toBe("Changes · 3");
});

it("shows the helper summary and copies a file patch", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { clipboard: { writeText } });
  render(<ChangesPanel workspaceId="project" refreshToken={1} />);
  expect(await screen.findByText(changesSummaryLabel(diffs.length))).toBeTruthy();
  const buttons = screen.getAllByRole("button", { name: "Copy" });
  expect(buttons).toHaveLength(diffs.length);
  fireEvent.click(buttons[0]!);
  await waitFor(() => expect(writeText).toHaveBeenCalledWith(diffs[0].patch));
  fireEvent.click(buttons[1]!);
  await waitFor(() => expect(writeText).toHaveBeenCalledWith(diffs[1].patch));
});

it("filters files by path substring", async () => {
  render(<ChangesPanel workspaceId="project" refreshToken={1} />);
  expect(await screen.findByText("src/a.rs")).toBeTruthy();
  expect(screen.getByText("notes.md")).toBeTruthy();
  fireEvent.change(screen.getByRole("textbox", { name: "Find a change" }), { target: { value: "notes" } });
  expect(screen.getByText("notes.md")).toBeTruthy();
  expect(screen.queryByText("src/a.rs")).toBeNull();
  expect(screen.queryByText("README.md")).toBeNull();
  expect(screen.getByText(changesSummaryLabel(diffs.length))).toBeTruthy();
});
