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
        { id: "github", displayName: "GitHub", status: "connected", accountLabel: "test", description: "Repositories", category: "Development", authKind: "device" },
        { id: "linear", displayName: "Linear", status: "available", description: "Issues", category: "Planning", authKind: "token" },
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

it("toggles a per-agent GitHub grant and resolves a pending approval", async () => {
  render(<Plugins />);
  fireEvent.click(await screen.findByRole("checkbox", { name: "Release manager" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_set_agent_grant", {
      agentId: "agent-1",
      pluginId: "github",
      enabled: true,
    }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Allow" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_resolve_approval", { id: "appr-1", allow: true }),
  );
});

it("opens a local credential flow and sends only the connection command", async () => {
  render(<Plugins />);
  const linear = (await screen.findByText("Linear")).closest("article");
  expect(linear).toBeTruthy();
  fireEvent.click(within(linear as HTMLElement).getByRole("button", { name: "Connect Linear" }));
  expect(await screen.findByRole("dialog", { name: "Connect Linear" })).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("dialog", { name: "Connect Linear" })).toBeNull();
  fireEvent.click(within(linear as HTMLElement).getByRole("button", { name: "Connect Linear" }));
  fireEvent.change(screen.getByPlaceholderText("Personal workspace"), { target: { value: "Product" } });
  fireEvent.change(screen.getByLabelText("Credential"), { target: { value: "local-test-token" } });
  fireEvent.click(screen.getByRole("button", { name: "Save connection" }));
  await waitFor(() =>
    expect(mocks.invoke).toHaveBeenCalledWith("plugin_configure", {
      id: "linear",
      credential: "local-test-token",
      accountLabel: "Product",
    }),
  );
});
