// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Settings } from "./Settings";
import { UI_ZOOM_PROPERTY } from "./ui-zoom";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

function mockIpc(get?: (key?: string) => unknown) {
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation((command: string, args?: { key?: string }) => {
    if (command === "settings_get") {
      const extra = get?.(args?.key);
      if (extra !== undefined) return Promise.resolve(extra);
      return Promise.resolve(null);
    }
    if (command === "settings_set") return Promise.resolve();
    if (command === "engines_detect") return Promise.resolve([]);
    if (command === "usage_bridge_status") return Promise.resolve({ connected: false, chained: false });
    return Promise.resolve(null);
  });
}

describe("Settings UI zoom", () => {
  afterEach(() => {
    document.documentElement.style.removeProperty(UI_ZOOM_PROPERTY);
    cleanup();
  });

  beforeEach(() => {
    mockIpc((key) => (key === "ui_zoom" ? "90" : undefined));
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

describe("Settings notification kinds", () => {
  afterEach(() => cleanup());

  beforeEach(() => {
    mockIpc();
  });

  it("saves the updated allow-list when a kind is toggled off", async () => {
    render(
      <Settings
        theme="black"
        initialPage="notifications"
        onThemeChange={() => undefined}
        onClose={() => undefined}
        onShowWelcome={() => undefined}
      />,
    );

    const mail = await screen.findByRole("switch", { name: "Mail" });
    await waitFor(() => expect(mail.getAttribute("aria-checked")).toBe("true"));
    fireEvent.click(mail);
    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith("settings_set", {
        key: "notification_kinds",
        value: [
          "permission",
          "terminal-exit",
          "turn-finished",
          "turn-cancelled",
          "turn-refused",
          "turn-stopped",
        ],
      });
    });
  });

  it("loads a stored allow-list", async () => {
    mockIpc((key) => (key === "notification_kinds" ? ["mail"] : undefined));

    render(
      <Settings
        theme="black"
        initialPage="notifications"
        onThemeChange={() => undefined}
        onClose={() => undefined}
        onShowWelcome={() => undefined}
      />,
    );

    const mail = await screen.findByRole("switch", { name: "Mail" });
    const needsYou = screen.getByRole("switch", { name: "Needs you" });
    await waitFor(() => expect(mail.getAttribute("aria-checked")).toBe("true"));
    await waitFor(() => expect(needsYou.getAttribute("aria-checked")).toBe("false"));
  });
});

describe("Settings default shell", () => {
  afterEach(cleanup);

  beforeEach(() => {
    mockIpc();
  });

  it("offers PowerShell and Command Prompt with the unix shells", async () => {
    render(
      <Settings
        theme="black"
        onThemeChange={() => undefined}
        onClose={() => undefined}
        onShowWelcome={() => undefined}
      />,
    );

    const select = await screen.findByRole("combobox", { name: "Default shell" });
    const options = Array.from((select as HTMLSelectElement).options);
    expect(options.map((option) => option.value)).toEqual(["system", "zsh", "bash", "powershell", "cmd"]);
    expect(options.map((option) => option.textContent)).toEqual([
      "System default",
      "zsh",
      "bash",
      "PowerShell",
      "Command Prompt",
    ]);

    fireEvent.change(select, { target: { value: "cmd" } });
    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith("settings_set", { key: "default_shell", value: "cmd" });
    });
  });
});

describe("Settings search", () => {
  afterEach(cleanup);

  beforeEach(() => {
    mockIpc();
  });

  function renderSettings(initialPage?: "general" | "notifications") {
    render(
      <Settings
        theme="black"
        initialPage={initialPage}
        onThemeChange={() => undefined}
        onClose={() => undefined}
        onShowWelcome={() => undefined}
      />,
    );
  }

  function searchPages() {
    return Array.from(document.querySelectorAll(".harbor-settings-search-group .harbor-eyebrow"), (node) => node.textContent);
  }

  it("finds Default shell from any page and restores the current page when cleared", async () => {
    renderSettings("notifications");

    expect(await screen.findByRole("switch", { name: "Mail" })).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: "Default shell" })).toBeNull();

    const field = screen.getByRole("textbox", { name: "Search settings" });
    expect(field.getAttribute("placeholder")).toBe("Find a setting");

    fireEvent.change(field, { target: { value: "shell" } });
    expect(screen.getByRole("combobox", { name: "Default shell" })).toBeTruthy();
    expect(searchPages()).toEqual(["General"]);
    expect(screen.queryByRole("switch", { name: "Mail" })).toBeNull();
    expect(screen.queryByText("Appearance")).toBeNull();
    expect(screen.queryByText("Make Harbor feel like your desk. Every preference is stored on this machine.")).toBeNull();

    fireEvent.change(field, { target: { value: "" } });
    expect(await screen.findByRole("switch", { name: "Mail" })).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: "Default shell" })).toBeNull();
  });

  it("finds Mail across pages and reports when nothing matches", async () => {
    renderSettings();

    const field = screen.getByRole("textbox", { name: "Search settings" });
    fireEvent.change(field, { target: { value: "Mail" } });

    expect(await screen.findByRole("switch", { name: "Mail" })).toBeTruthy();
    expect(screen.getByRole("switch", { name: "Pause agent mail" })).toBeTruthy();
    expect(searchPages()).toEqual(["Notifications", "Agents"]);
    expect(screen.queryByRole("combobox", { name: "Default shell" })).toBeNull();

    fireEvent.change(field, { target: { value: "xyzzy" } });
    expect(screen.getByText("No settings match “xyzzy”.")).toBeTruthy();
    expect(screen.queryByRole("switch", { name: "Mail" })).toBeNull();
  });
});
