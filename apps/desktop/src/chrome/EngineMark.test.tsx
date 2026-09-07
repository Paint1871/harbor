// @vitest-environment jsdom
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { EngineMark, resolveEngineId } from "./EngineMark";

describe("EngineMark", () => {
  afterEach(() => cleanup());

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
});
