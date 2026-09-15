// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { NewAgent } from "./NewAgent";

const mocks = vi.hoisted(() => ({
  call: vi.fn(),
  settingsGet: vi.fn(),
}));

vi.mock("../../ipc", () => ({ call: mocks.call }));
vi.mock("../../settings", () => ({
  settingsGet: mocks.settingsGet,
  settingsSet: vi.fn(),
}));
vi.mock("./Face", async () => {
  const React = await import("react");
  return {
    Face: ({ name }: { name: string }) =>
      React.createElement("span", { className: "harbor-face" }, name),
  };
});

beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value() {
      this.open = true;
    },
  });
  mocks.call.mockReset();
  mocks.settingsGet.mockResolvedValue(null);
  mocks.call.mockImplementation((command: string) => {
    if (command === "engines_detect") {
      return Promise.resolve([{
        id: "opencode",
        displayName: "OpenCode",
        path: "/usr/local/bin/opencode",
        status: "ready",
        supportsChat: true,
        supportsTerminal: true,
      }]);
    }
    return Promise.resolve(null);
  });
});

afterEach(cleanup);

it("omits faceIndex when the picker is never used", async () => {
  const onCreate = vi.fn().mockResolvedValue(undefined);
  render(<NewAgent onCreate={onCreate} onClose={vi.fn()} />);
  await screen.findByRole("option", { name: "OpenCode" });
  fireEvent.change(screen.getByPlaceholderText("e.g. Release partner"), {
    target: { value: "Mate" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Create agent" }));
  await waitFor(() => expect(onCreate).toHaveBeenCalled());
  expect(onCreate.mock.calls[0]?.[0]).toEqual({
    name: "Mate",
    brief: "",
    engineId: "opencode",
    homePath: undefined,
  });
  expect(onCreate.mock.calls[0]?.[0].faceIndex).toBeUndefined();
});

it("sends the picked face when FacePicker is used", async () => {
  const onCreate = vi.fn().mockResolvedValue(undefined);
  render(<NewAgent onCreate={onCreate} onClose={vi.fn()} />);
  await screen.findByRole("option", { name: "OpenCode" });
  fireEvent.change(screen.getByPlaceholderText("e.g. Release partner"), {
    target: { value: "Mate" },
  });
  const faces = screen.getByRole("listbox", { name: "Face" }).querySelectorAll("button");
  fireEvent.click(faces[3]!);
  fireEvent.click(screen.getByRole("button", { name: "Create agent" }));
  await waitFor(() => expect(onCreate).toHaveBeenCalled());
  expect(onCreate.mock.calls[0]?.[0].faceIndex).toBe(3);
});
