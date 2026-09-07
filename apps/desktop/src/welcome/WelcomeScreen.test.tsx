// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import { WelcomeScreen } from "./WelcomeScreen";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

describe("WelcomeScreen", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mocks.invoke.mockReset();
    // jsdom has no matchMedia; ThemeProvider subscribes to reduced motion.
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }));
  });

  it("starts local with the OS account name, not a hardcoded persona", async () => {
    mocks.invoke.mockResolvedValue("Lennox");
    const onStartLocal = vi.fn();
    render(
      <ThemeProvider theme="black">
        <WelcomeScreen onStartLocal={onStartLocal} />
      </ThemeProvider>,
    );

    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("default_profile_name"));
    fireEvent.click(screen.getAllByRole("button", { name: "Start local" })[0]);
    await waitFor(() => expect(onStartLocal).toHaveBeenCalledWith("Lennox"));
  });

  it("falls back when the host cannot supply an account name", async () => {
    mocks.invoke.mockRejectedValue(new Error("no host"));
    const onStartLocal = vi.fn();
    render(
      <ThemeProvider theme="black">
        <WelcomeScreen onStartLocal={onStartLocal} />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getAllByRole("button", { name: "Start local" })[0]);
    await waitFor(() => expect(onStartLocal).toHaveBeenCalledWith("Local"));
  });
});
