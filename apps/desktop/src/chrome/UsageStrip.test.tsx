// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { UsageStrip, untilReset, windowName } from "./UsageStrip";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const now = () => Math.floor(Date.now() / 1000);
/** Keeps the floored countdown off a boundary a second of test drift could cross. */
const SLACK = 30;

function usage(windows: { windowMinutes: number; usedPercent: number; resetsAt: number }[]) {
  return [{ engineId: "codex", displayName: "Codex", windows, measuredAt: now() }];
}

describe("UsageStrip", () => {
  afterEach(cleanup);
  beforeEach(() => mocks.invoke.mockReset());

  it("reads each window as a percentage and the time left on it", async () => {
    mocks.invoke.mockResolvedValue(
      usage([
        { windowMinutes: 300, usedPercent: 36.4, resetsAt: now() + 2 * 3600 + 55 * 60 + SLACK },
        { windowMinutes: 10080, usedPercent: 14, resetsAt: now() + 4 * 86400 + 17 * 3600 + SLACK },
      ]),
    );
    render(<UsageStrip />);

    await screen.findByText("Codex");
    expect(screen.getByText("36% used")).toBeTruthy();
    expect(screen.getByText("2h 55m")).toBeTruthy();
    expect(screen.getByText("14% used")).toBeTruthy();
    expect(screen.getByText("4d 17h")).toBeTruthy();
    expect(screen.getByRole("meter", { name: "5h limit" }).getAttribute("aria-valuenow")).toBe("36");
  });

  it("stays out of the rail when no engine publishes a live limit", async () => {
    mocks.invoke.mockResolvedValue(usage([{ windowMinutes: 300, usedPercent: 80, resetsAt: now() - 60 }]));
    const { container } = render(<UsageStrip />);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("engine_usage"));
    expect(container.querySelector(".harbor-usage")).toBeNull();

    cleanup();
    mocks.invoke.mockResolvedValue([]);
    const empty = render(<UsageStrip />).container;
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(2));
    expect(empty.querySelector(".harbor-usage")).toBeNull();
  });

  it("re-reads the engines' files when asked", async () => {
    mocks.invoke.mockResolvedValue(usage([{ windowMinutes: 300, usedPercent: 36, resetsAt: now() + 3600 }]));
    render(<UsageStrip />);
    await screen.findByText("36% used");

    mocks.invoke.mockResolvedValue(usage([{ windowMinutes: 300, usedPercent: 41, resetsAt: now() + 3600 }]));
    fireEvent.click(screen.getByRole("button", { name: "Refresh usage" }));
    await screen.findByText("41% used");
  });

  it("picks up usage that only appears after it was first read", async () => {
    mocks.invoke.mockResolvedValue([]);
    const { container } = render(<UsageStrip />);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledTimes(1));
    expect(container.querySelector(".harbor-usage")).toBeNull();

    // The bridge gets connected, or a CLI runs, while Harbor is already open.
    mocks.invoke.mockResolvedValue(usage([{ windowMinutes: 300, usedPercent: 15, resetsAt: now() + 3600 }]));
    fireEvent.focus(window);
    await screen.findByText("15% used");
  });

  it("names the windows the CLIs use and keeps the countdown coarse", () => {
    expect(windowName(300)).toBe("5h");
    expect(windowName(10080)).toBe("Weekly");
    expect(untilReset(1000 + 45, 1000)).toBe("0m");
    expect(untilReset(1000 + 3 * 86400 + 2 * 3600, 1000)).toBe("3d 2h");
    expect(untilReset(500, 1000)).toBe("0m");
  });
});
