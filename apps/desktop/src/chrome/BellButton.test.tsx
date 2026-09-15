// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BellButton } from "./BellButton";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  play: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("./notificationBeep", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./notificationBeep")>();
  return { ...actual, playNotificationBeep: mocks.play };
});

function setHidden(hidden: boolean) {
  Object.defineProperty(document, "hidden", { configurable: true, value: hidden });
}

function mockHost(sound: unknown, unread = 0) {
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "notifications_unread_count") return Promise.resolve(unread);
    if (command === "settings_get") return Promise.resolve(sound);
    return Promise.resolve();
  });
}

async function fireNotification() {
  await waitFor(() => expect(mocks.listen).toHaveBeenCalledWith("notification", expect.any(Function)));
  const handler = mocks.listen.mock.calls[0][1] as () => void;
  handler();
}

describe("BellButton", () => {
  afterEach(() => {
    cleanup();
    setHidden(false);
  });
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
    mocks.play.mockReset();
    mocks.listen.mockResolvedValue(() => undefined);
    mockHost(false, 0);
    setHidden(false);
  });

  it("renders without throwing when Web Audio is missing", () => {
    expect(() => render(<BellButton onClick={() => undefined} />)).not.toThrow();
    expect(screen.getByRole("button", { name: "Notifications" })).toBeTruthy();
  });

  it("shows the unread count the host reports", async () => {
    mockHost(false, 3);
    render(<BellButton onClick={() => undefined} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "Notifications, 3 unread" })).toBeTruthy());
  });

  it("does not beep while Harbor is in the foreground", async () => {
    mockHost(true, 1);
    setHidden(false);
    render(<BellButton onClick={() => undefined} />);
    await fireNotification();
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("settings_get", { key: "notification_sound" }));
    expect(mocks.play).not.toHaveBeenCalled();
  });

  it("does not beep when the sound setting is off", async () => {
    mockHost(false, 1);
    setHidden(true);
    render(<BellButton onClick={() => undefined} />);
    await fireNotification();
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("settings_get", { key: "notification_sound" }));
    expect(mocks.play).not.toHaveBeenCalled();
  });

  it("beeps for a background event when the sound setting is on", async () => {
    mockHost(true, 1);
    setHidden(true);
    render(<BellButton onClick={() => undefined} />);
    await fireNotification();
    await waitFor(() => expect(mocks.play).toHaveBeenCalled());
  });
});
