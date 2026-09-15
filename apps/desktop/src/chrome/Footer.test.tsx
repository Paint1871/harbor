// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ChromeProvider, type ChromeValue } from "./chrome-context";
import { Footer } from "./Footer";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => []) }));

const chrome = (over: Partial<ChromeValue> = {}): ChromeValue => ({
  mode: "agent",
  theme: "black",
  profileName: "Ada",
  destination: "mode",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onOpenSession: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
  registerNewThread: () => undefined,
  registerNewAgentChat: () => undefined,
  registerCodeWorkspace: () => undefined,
  ...over,
});

function openAppearance() {
  fireEvent.click(screen.getByRole("button", { name: "Appearance" }));
  expect(screen.getByRole("radiogroup", { name: "Appearance" })).toBeTruthy();
  expect(screen.getByRole("radio", { name: "Black" })).toBeTruthy();
  expect(screen.getByRole("radio", { name: "Light" })).toBeTruthy();
}

afterEach(cleanup);
beforeEach(() => {
  render(
    <ChromeProvider value={chrome()}>
      <Footer />
    </ChromeProvider>,
  );
});

it("closes the appearance menu on Escape", () => {
  openAppearance();
  fireEvent.keyDown(window, { key: "Escape", bubbles: true });
  expect(screen.queryByRole("radiogroup", { name: "Appearance" })).toBeNull();
  expect(screen.queryByRole("radio", { name: "Black" })).toBeNull();
  expect(screen.queryByRole("radio", { name: "Light" })).toBeNull();
});

it("closes the appearance menu on pointer down outside", () => {
  openAppearance();
  fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("radiogroup", { name: "Appearance" })).toBeNull();
  expect(screen.queryByRole("radio", { name: "Black" })).toBeNull();
  expect(screen.queryByRole("radio", { name: "Light" })).toBeNull();
});
