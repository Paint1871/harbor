// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Workspace } from "@harbor/schema/commands";
import { WorkspaceRailRow } from "./WorkspaceRailRow";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const workspace: Workspace = {
  id: "w1",
  folder: "/Users/x/Free Project",
  title: "Free Project",
  pinned: false,
};

function renderRow(onRenamed = vi.fn(), onSelect = vi.fn(), onRemoved?: () => void) {
  render(
    <WorkspaceRailRow
      workspace={workspace}
      onSelect={onSelect}
      onRenamed={onRenamed}
      onRemoved={onRemoved}
    />,
  );
  return { onRenamed, onSelect, onRemoved };
}

describe("WorkspaceRailRow", () => {
  afterEach(() => cleanup());
  beforeEach(() => mocks.invoke.mockReset());

  it("renames on right click and reports the stored workspace", async () => {
    const renamed = { ...workspace, title: "Launch work" };
    mocks.invoke.mockResolvedValue(renamed);
    const { onRenamed } = renderRow();

    fireEvent.contextMenu(screen.getByRole("button", { name: /Free Project/ }));
    const input = screen.getByLabelText("Rename Free Project");
    fireEvent.change(input, { target: { value: "Launch work" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith("workspace_rename", { id: "w1", title: "Launch work" }),
    );
    await waitFor(() => expect(onRenamed).toHaveBeenCalledWith(renamed));
  });

  it("opens the editor with F2 and cancels on Escape without saving", async () => {
    const { onRenamed } = renderRow();
    const row = screen.getByRole("button", { name: /Free Project/ });
    fireEvent.keyDown(row, { key: "F2" });

    const input = screen.getByLabelText("Rename Free Project");
    fireEvent.change(input, { target: { value: "Discarded" } });
    fireEvent.keyDown(input, { key: "Escape" });

    await waitFor(() => expect(screen.getByRole("button", { name: /Free Project/ })).toBeTruthy());
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(onRenamed).not.toHaveBeenCalled();
  });

  it("keeps the editor open and shows the host error when the rename is rejected", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "workspace_rename"
        ? Promise.reject(new Error("Use 60 characters or fewer."))
        : Promise.resolve([]),
    );
    renderRow();
    fireEvent.contextMenu(screen.getByRole("button", { name: /Free Project/ }));
    const input = screen.getByLabelText("Rename Free Project");
    fireEvent.change(input, { target: { value: "x".repeat(61) } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("60 characters"));
    expect(screen.getByLabelText("Rename Free Project")).toBeTruthy();
  });

  it("selects the workspace on a single click", () => {
    const { onSelect } = renderRow();
    fireEvent.click(screen.getByRole("button", { name: /Free Project/ }));
    expect(onSelect).toHaveBeenCalled();
  });

  it("removes the folder only after the ask, and leaves the row alone without a handler", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    const onRemoved = vi.fn();
    renderRow(vi.fn(), vi.fn(), onRemoved);
    fireEvent.contextMenu(screen.getByRole("button", { name: /Free Project/ }));

    const remove = screen.getByLabelText("Remove Free Project from Workspaces");
    fireEvent.click(remove);
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(remove.textContent).toContain("Remove?");

    fireEvent.click(remove);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("workspace_remove", { id: "w1" }));
    await waitFor(() => expect(onRemoved).toHaveBeenCalledWith("w1"));

    cleanup();
    renderRow();
    fireEvent.contextMenu(screen.getByRole("button", { name: /Free Project/ }));
    expect(screen.queryByLabelText("Remove Free Project from Workspaces")).toBeNull();
  });

  it("keeps the editor open when focus moves to Remove, instead of committing the rename", () => {
    const { onRenamed } = renderRow(vi.fn(), vi.fn(), vi.fn());
    fireEvent.contextMenu(screen.getByRole("button", { name: /Free Project/ }));
    const input = screen.getByLabelText("Rename Free Project");
    fireEvent.change(input, { target: { value: "Half typed" } });
    fireEvent.blur(input, { relatedTarget: screen.getByLabelText("Remove Free Project from Workspaces") });

    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(onRenamed).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Rename Free Project")).toBeTruthy();
  });

  it("does not open the editor on a double click, which the row uses to expand", () => {
    const { onSelect } = renderRow();
    const row = screen.getByRole("button", { name: /Free Project/ });
    fireEvent.doubleClick(row);
    expect(screen.queryByLabelText("Rename Free Project")).toBeNull();
    expect(onSelect).not.toHaveBeenCalledWith(expect.anything());
  });
});
