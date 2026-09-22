// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Plugins } from "./Plugins";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => undefined),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "plugin_list") {
      return Promise.resolve([
        { id: "github", displayName: "GitHub", status: "available", description: "Repositories", category: "Development", authKind: "token" },
        { id: "linear", displayName: "Linear", status: "soon", description: "Issues", category: "Planning", authKind: "token" },
        { id: "stripe", displayName: "Stripe", status: "connected", accountLabel: "test", description: "Customers", category: "Commerce", authKind: "token" },
      ]);
    }
    if (command === "agent_list") {
      return Promise.resolve([{ id: "agent-1", name: "Release manager", brief: "", engineId: "opencode", faceIndex: 0, pinned: false }]);
    }
    if (command === "plugin_grants_list") return Promise.resolve([]);
    if (command === "plugin_approvals_list") {
      return Promise.resolve([{ id: "appr-1", pluginId: "github", agentId: "agent-1", action: "write", status: "pending" }]);
    }
    return Promise.resolve();
  });
});
afterEach(cleanup);

it("toggles a per-agent grant on a connected row and resolves a pending approval", async () => {
  render(<Plugins />);
  fireEvent.click(await screen.findByRole("checkbox", { name: "Release manager · Stripe" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_set_agent_grant", {
      agentId: "agent-1",
      pluginId: "stripe",
      enabled: true,
    }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Allow" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_resolve_approval", { id: "appr-1", allow: true }),
  );
});

it("surfaces a failed resolution instead of silently dropping it", async () => {
  render(<Plugins />);
  await screen.findByRole("button", { name: "Allow" });
  const approvalsLoaded = () =>
    mocks.invoke.mock.calls.filter(([command]) => command === "plugin_approvals_list").length;
  const loaded = approvalsLoaded();
  mocks.invoke.mockImplementation((command: string) =>
    command === "plugin_resolve_approval"
      ? Promise.reject("database locked")
      : Promise.resolve(command === "plugin_list" ? [] : undefined),
  );
  fireEvent.click(screen.getByRole("button", { name: "Allow" }));
  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toContain("could not be applied");
  // No reload ran — the row is still pending and the card stays put.
  expect(approvalsLoaded()).toBe(loaded);
  expect(screen.getByRole("button", { name: "Allow" })).toBeTruthy();
});

it("starts the GitHub device flow from Connect", async () => {
  render(<Plugins />);
  fireEvent.click(await screen.findByRole("button", { name: "Connect GitHub" }));
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("plugin_connect", { id: "github" }));
  expect(mocks.invoke).not.toHaveBeenCalledWith("plugin_configure", expect.anything());
});

it("keeps the token form as the explicit GitHub fallback", async () => {
  render(<Plugins />);
  fireEvent.click(await screen.findByRole("button", { name: "Connect GitHub with a token" }));
  expect(await screen.findByRole("dialog", { name: "Connect GitHub" })).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog", { name: "Connect GitHub" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Connect GitHub with a token" }));
  fireEvent.change(screen.getByPlaceholderText("Personal workspace"), { target: { value: "Product" } });
  fireEvent.change(screen.getByLabelText("Credential"), { target: { value: "local-test-token" } });
  fireEvent.click(screen.getByRole("button", { name: "Save connection" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_configure", {
      id: "github",
      credential: "local-test-token",
      accountLabel: "Product",
    }),
  );
});

it("marks unreleased catalog entries as soon without a connect action", async () => {
  render(<Plugins />);
  const linear = (await screen.findByText("Linear")).closest("article");
  expect(linear).toBeTruthy();
  const button = within(linear as HTMLElement).getByRole("button", { name: "Linear is not available yet" });
  expect(button).toHaveProperty("disabled", true);
  fireEvent.click(button);
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(mocks.invoke).not.toHaveBeenCalledWith("plugin_connect", { id: "linear" });
  expect(mocks.invoke).not.toHaveBeenCalledWith("plugin_configure", expect.objectContaining({ id: "linear" }));
});
