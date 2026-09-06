// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ChromeProvider, type ChromeValue } from "../chrome/chrome-context";
import { Skills } from "./Skills";

const settingsSet = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("../settings", () => ({ settingsSet }));

const onModeChange = vi.fn();
const chrome: ChromeValue = {
  mode: "agent",
  theme: "black",
  profileName: "Local",
  destination: "skills",
  setDestination: () => undefined,
  onModeChange,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
};

afterEach(() => {
  cleanup();
  onModeChange.mockReset();
  settingsSet.mockClear();
});

it("loads a playbook into the Agent composer", async () => {
  render(<ChromeProvider value={chrome}><Skills /></ChromeProvider>);
  fireEvent.click(screen.getByRole("button", { name: /Release notes/ }));
  fireEvent.click(screen.getByRole("button", { name: "Use in Agent" }));
  await waitFor(() => {
    expect(settingsSet).toHaveBeenCalledWith("pending_agent_prompt", expect.stringContaining("release notes"));
    expect(onModeChange).toHaveBeenCalledWith("agent");
  });
});
