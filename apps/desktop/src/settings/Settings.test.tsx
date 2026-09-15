// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Settings } from "./Settings";
import { UI_ZOOM_PROPERTY } from "./ui-zoom";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

describe("Settings UI zoom", () => {
  afterEach(() => {
    document.documentElement.style.removeProperty(UI_ZOOM_PROPERTY);
    cleanup();
  });

  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation((command: string, args?: { key?: string }) => {
      if (command === "settings_get" && args?.key === "ui_zoom") return Promise.resolve("90");
      if (command === "settings_get") return Promise.resolve(null);
      if (command === "settings_set") return Promise.resolve();
      if (command === "engines_detect") return Promise.resolve([]);
      if (command === "usage_bridge_status") return Promise.resolve({ connected: false, chained: false });
      return Promise.resolve(null);
    });
  });

  it("loads the stored percent and applies a new one immediately", async () => {
    render(
      <Settings
        theme="black"
        onThemeChange={() => undefined}
        onClose={() => undefined}
        onShowWelcome={() => undefined}
      />,
    );

    const select = await screen.findByRole("combobox", { name: "UI zoom" });
    await waitFor(() => expect((select as HTMLSelectElement).value).toBe("90"));

    fireEvent.change(select, { target: { value: "125" } });
    expect(document.documentElement.style.getPropertyValue(UI_ZOOM_PROPERTY)).toBe("1.25");
    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith("settings_set", { key: "ui_zoom", value: "125" });
    });
  });
});
