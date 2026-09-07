// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EngineMark, resetEngineIconsForTest, resolveEngineId } from "./EngineMark";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

describe("EngineMark", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(() => Promise.resolve([]));
    resetEngineIconsForTest();
  });
  afterEach(() => {
    cleanup();
    resetEngineIconsForTest();
  });

  it("keeps the mark in step with a label when the engine id is missing", () => {
    expect(resolveEngineId(undefined, "Claude Code")).toBe("claude-code");
    expect(resolveEngineId(null, "Codex")).toBe("codex");
    expect(resolveEngineId("shell", "zsh 104×31")).toBe("shell");
    expect(resolveEngineId("claude-code", "Claude Code")).toBe("claude-code");
  });

  it("gives two different engines two different marks", () => {
    const { container: a } = render(<EngineMark engineId="claude-code" label="Claude Code" />);
    const { container: b } = render(<EngineMark engineId="codex" label="Codex" />);
    expect(a.innerHTML).not.toBe(b.innerHTML);
    expect(a.querySelector("svg")).toBeTruthy();
    expect(b.querySelector("svg")).toBeTruthy();
  });

  it("falls back to a monogram tile for an engine with no drawn mark", () => {
    const { container } = render(<EngineMark engineId="antigravity" label="Antigravity" />);
    expect(container.querySelector(".harbor-engine-monogram")?.textContent).toBe("AN");
    const { container: two } = render(<EngineMark engineId="muse-code" label="Muse Code" />);
    expect(two.querySelector(".harbor-engine-monogram")?.textContent).toBe("MC");
  });

  it("prefers an installed vendor logo over the drawn mark", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "engine_icons"
        ? Promise.resolve([{ engineId: "codex", dataUrl: "data:image/svg+xml;base64,PHN2Zy8+" }])
        : Promise.resolve([]),
    );
    const { container } = render(<EngineMark engineId="codex" label="Codex" />);
    await waitFor(() =>
      expect(container.querySelector("img")?.getAttribute("src")).toBe(
        "data:image/svg+xml;base64,PHN2Zy8+",
      ),
    );
    // An engine with no installed logo keeps Harbor's own mark.
    const { container: drawn } = render(<EngineMark engineId="cursor" label="Cursor" />);
    expect(drawn.querySelector("img")).toBeNull();
    expect(drawn.querySelector("svg")).toBeTruthy();
  });
});
