// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { Tree } from "./Tree";

const listWorkspace = vi.hoisted(() => vi.fn());
vi.mock("./fs", () => ({
  listWorkspace,
  filesystemError: (reason: unknown, fallback: string) => String(reason) || fallback,
}));

afterEach(() => {
  cleanup();
  listWorkspace.mockReset();
});

it("filters the listing from Search files and hides a non-matching name", async () => {
  const onOpen = vi.fn();
  listWorkspace.mockImplementation((_workspaceId: string, path: string) => {
    if (path === "src") {
      return Promise.resolve([{ name: "lib.rs", path: "/tmp/src/lib.rs", directory: false }]);
    }
    return Promise.resolve([
      { name: "src", path: "src", directory: true },
      { name: "main.rs", path: "/tmp/main.rs", directory: false },
      { name: "README.md", path: "/tmp/README.md", directory: false },
    ]);
  });

  render(<Tree workspaceId="ws" onOpen={onOpen} />);

  const field = await screen.findByRole("textbox", { name: "Search files" });
  expect(field.getAttribute("placeholder")).toBe("Find a file");
  await waitFor(() => {
    expect(screen.getByRole("button", { name: "▸ src" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "main.rs" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "README.md" })).toBeTruthy();
  });

  fireEvent.change(field, { target: { value: "main" } });
  expect(screen.getByRole("button", { name: "main.rs" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "README.md" })).toBeNull();
  expect(screen.queryByRole("button", { name: "▸ src" })).toBeNull();

  fireEvent.click(screen.getByRole("button", { name: "main.rs" }));
  expect(onOpen).toHaveBeenCalledWith("/tmp/main.rs");

  fireEvent.change(field, { target: { value: "src" } });
  expect(screen.getByRole("button", { name: "▸ src" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "main.rs" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "▸ src" }));
  expect(await screen.findByRole("button", { name: "▾ src" })).toBeTruthy();

  fireEvent.change(field, { target: { value: "xyzzy" } });
  expect(screen.getByText("No files match “xyzzy”.")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "main.rs" })).toBeNull();
  expect(screen.queryByRole("button", { name: "README.md" })).toBeNull();

  fireEvent.change(field, { target: { value: "" } });
  expect(screen.getByRole("button", { name: "main.rs" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "README.md" })).toBeTruthy();
});
