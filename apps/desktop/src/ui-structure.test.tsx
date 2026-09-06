import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import { DesktopShell } from "./chrome/DesktopShell";
import { ChromeProvider, type ChromeValue } from "./chrome/chrome-context";
import { AgentRail } from "./modes/agent/AgentRail";
import { AgentPage } from "./modes/agent/AgentPage";
import { Transcript } from "./chrome/Transcript";
import { PaneHeader } from "./panes/PaneHeader";
import type { AgentRecord } from "@harbor/schema/commands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: async () => [],
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => undefined,
}));
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    loadAddon() {}
    open() {}
    writeln() {}
    write() {}
    dispose() {}
    onData() {
      return { dispose() {} };
    }
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
  },
}));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));
vi.mock("./modes/agent/Face", async () => {
  const React = await import("react");
  return {
    Face: ({ name }: { name: string }) =>
      React.createElement("span", { className: "harbor-face" }, name),
  };
});

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");

const chromeStub: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Builder",
  destination: "mode",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
};

function wrap(node: ReactNode) {
  return renderToStaticMarkup(
    createElement(ChromeProvider, { value: chromeStub }, node),
  );
}

const teammate: AgentRecord = {
  id: "agent-1",
  name: "Release manager",
  brief: "Ship the launch",
  engineId: "opencode",
  faceIndex: 0,
  pinned: false,
};

describe("shipped Harbor chrome and mode trees", () => {
  it("renders Agent, Code, Chat, destinations, roster, New chat, panes, and a user bubble", () => {
    const shell = renderToStaticMarkup(
      createElement(
        ThemeProvider,
        { theme: "black" },
        createElement(DesktopShell, {
          theme: "black",
          onThemeChange: () => undefined,
          profileName: "Builder",
        }),
      ),
    );
    expect(shell).toContain("Agent");
    expect(shell).toContain("Code");
    expect(shell).toContain("Chat");
    expect(shell).not.toContain("Run in the focused terminal");
    expect(shell).not.toContain('aria-label="Terminal command"');
    expect(shell).toContain("Dashboard");
    expect(shell).toContain("Routines");
    expect(shell).toContain("Plugins");
    expect(shell).toContain("Skills");
    expect(shell).toContain("Harbor");
    expect(shell).toContain("Credits");

    const agentTree = wrap(
      createElement(
        "div",
        null,
        createElement(AgentRail, {
          agents: [teammate],
          selectedId: teammate.id,
          onSelect: () => undefined,
          onNew: () => undefined,
        }),
        createElement(AgentPage, { agent: teammate }),
      ),
    );
    expect(agentTree).toContain("Release manager");
    expect(agentTree).toContain("New Agent");
    expect(agentTree).toContain("New chat");
    expect(agentTree).toContain("harbor-roster");
    expect(agentTree).toContain('aria-current="true"');

    const codeHeader = wrap(
      createElement("div", null, [
        createElement(PaneHeader, { key: "t", title: "Terminal", live: true, onExpand: () => undefined }),
        createElement(PaneHeader, { key: "f", title: "Files", live: false, onExpand: () => undefined }),
      ]),
    );
    expect(codeHeader).toContain("harbor-live-dot");
    expect(codeHeader).toContain("Terminal");
    expect(codeHeader).toContain("Files");
    expect(codeHeader).toContain("Split");
    expect(codeHeader).toContain("Close");

    const chatTree = wrap(
      createElement(Transcript, {
        lines: [
          { id: "u1", text: "Find launch leads", role: "user" },
          { id: "a1", text: "Here is a list.", role: "assistant" },
        ],
      }),
    );
    expect(chatTree).toContain("harbor-bubble-user");
    expect(chatTree).toContain("Find launch leads");
    expect(chatTree).toContain("harbor-assistant-block");
  });

  it("keeps screenshot tokens and the composer placeholder in shipped files", () => {
    const tokens = readFileSync(resolve(repo, "packages/ui/src/tokens.css"), "utf8");
    const composer = readFileSync(resolve(repo, "packages/ui/src/Composer.tsx"), "utf8");
    expect(tokens).toContain("--harbor-bg: #0B0B0C");
    expect(tokens).toContain("--harbor-rail-w: 260px");
    expect(tokens).toContain("--harbor-titlebar-h: 44px");
    expect(composer).toContain('placeholder="Ask anything..."');
  });
});
