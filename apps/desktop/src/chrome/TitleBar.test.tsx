// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HostPlatform } from "../platform";

const platform = vi.hoisted(() => ({
  hostPlatform: vi.fn((): HostPlatform => "linux"),
}));
vi.mock("../platform", () => ({
  hostPlatform: () => platform.hostPlatform(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: async () => 0 }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => undefined }));

import { TitleBar } from "./TitleBar";

const css = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "../app.css"), "utf8");

const props = {
  mode: "agent" as const,
  onModeChange: () => undefined,
  railOpen: true,
  onToggleRail: () => undefined,
  onTidy: () => undefined,
  onBellClick: () => undefined,
  inboxOpen: false,
};

describe("TitleBar", () => {
  afterEach(cleanup);

  beforeEach(() => {
    platform.hostPlatform.mockReturnValue("linux");
  });

  it("treats Linux as an in-content toolbar without caption buttons or a macOS pad", () => {
    render(<TitleBar {...props} />);
    const bar = document.querySelector(".harbor-titlebar");
    expect(bar?.getAttribute("data-platform")).toBe("linux");
    expect(bar?.getAttribute("data-chrome")).toBe("in-content");
    expect(bar?.className).not.toContain("78px");
    expect(screen.queryByRole("button", { name: "Minimize" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Maximize" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Close" })).toBeNull();
    expect(screen.getByRole("radiogroup", { name: "Mode" })).toBeTruthy();
  });

  it("keeps Windows caption buttons and does not mark the bar in-content", () => {
    platform.hostPlatform.mockReturnValue("windows");
    render(<TitleBar {...props} />);
    const bar = document.querySelector(".harbor-titlebar");
    expect(bar?.getAttribute("data-platform")).toBe("windows");
    expect(bar?.getAttribute("data-chrome")).toBeNull();
    expect(screen.getByRole("button", { name: "Minimize" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Maximize" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Close" })).toBeTruthy();
  });

  it("leaves macOS overlay traffic lights to the native title bar", () => {
    platform.hostPlatform.mockReturnValue("macos");
    render(<TitleBar {...props} />);
    const bar = document.querySelector(".harbor-titlebar");
    expect(bar?.getAttribute("data-platform")).toBe("macos");
    expect(bar?.getAttribute("data-chrome")).not.toBe("in-content");
    expect(screen.queryByRole("button", { name: "Minimize" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Maximize" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Close" })).toBeNull();
  });
});

describe("title bar CSS", () => {
  it("pads macOS for traffic lights and keeps Linux in-content", () => {
    expect(css).toContain(
      '.harbor-app[data-platform="macos"] .harbor-titlebar { padding-left: 78px; }',
    );
    const linuxStart = css.indexOf('.harbor-app[data-platform="linux"] .harbor-titlebar');
    expect(linuxStart).toBeGreaterThan(-1);
    const linuxBlock = css.slice(linuxStart, css.indexOf("}", linuxStart) + 1);
    expect(linuxBlock).not.toContain("78px");
    expect(linuxBlock).toContain("data-chrome=\"in-content\"");
    expect(css).toContain(
      ".harbor-app[data-platform=\"linux\"] .harbor-window-controls {\n  display: none;\n}",
    );
  });
});
