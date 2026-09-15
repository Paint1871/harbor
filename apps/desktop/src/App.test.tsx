// @vitest-environment jsdom
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { UI_ZOOM_PROPERTY } from "./settings/ui-zoom";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => undefined }));
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

function stubMatchMedia() {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
}

function settings(values: Record<string, unknown>) {
  mocks.invoke.mockImplementation((command: string, args?: { key?: string }) => {
    if (command === "settings_get") return Promise.resolve(values[args?.key ?? ""] ?? null);
    if (command === "default_profile_name") return Promise.resolve("Local");
    return Promise.resolve(null);
  });
}

describe("App UI zoom", () => {
  afterEach(() => {
    document.documentElement.style.removeProperty(UI_ZOOM_PROPERTY);
    cleanup();
  });

  beforeEach(() => {
    mocks.invoke.mockReset();
    stubMatchMedia();
  });

  it("applies the saved zoom before the welcome screen paints", async () => {
    settings({ ui_zoom: "110", onboarded_local: false });
    render(<App />);
    await waitFor(() => {
      expect(document.documentElement.style.getPropertyValue(UI_ZOOM_PROPERTY)).toBe("1.1");
    });
    expect(document.querySelector(".harbor-welcome")).toBeTruthy();
  });

  it("uses 100% when the stored zoom is invalid", async () => {
    settings({ ui_zoom: "huge", onboarded_local: false });
    render(<App />);
    await waitFor(() => {
      expect(document.documentElement.style.getPropertyValue(UI_ZOOM_PROPERTY)).toBe("1");
    });
  });
});
