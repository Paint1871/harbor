// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { BrowserPane, createBrowserPaneState } from "./BrowserPane";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

afterEach(cleanup);

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockResolvedValue(undefined);
});

function renderPane(url = "https://example.com/app") {
  const state = { ...createBrowserPaneState(), url, draft: url.replace(/^https?:\/\//i, "") };
  render(
    <BrowserPane
      state={state}
      onStateChange={() => undefined}
      focused
      onFocus={() => undefined}
    />,
  );
}

it("opens the current preview URL in the system browser", () => {
  renderPane("https://example.com/app");
  fireEvent.click(screen.getByRole("button", { name: "Open in system browser" }));
  expect(mocks.invoke).toHaveBeenCalledWith("open_external_url", { url: "https://example.com/app" });
});

it("shows a short status when the host cannot open the URL", async () => {
  mocks.invoke.mockRejectedValue("only http and https URLs can open in the system browser");
  renderPane("https://example.com/app");
  fireEvent.click(screen.getByRole("button", { name: "Open in system browser" }));
  expect(mocks.invoke).toHaveBeenCalledWith("open_external_url", { url: "https://example.com/app" });
  expect(await screen.findByText("Could not open in system browser")).toBeTruthy();
});
