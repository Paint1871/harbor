// @vitest-environment jsdom
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Inbox } from "./Inbox";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

describe("Inbox", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  it("shows an empty state instead of invented events", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([]) : Promise.resolve(),
    );
    render(<Inbox open />);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("notifications_list"));
    expect(screen.getByText("Nothing yet. Agent events land here.")).toBeTruthy();
    expect(screen.queryByText(/failed$/)).toBeNull();
  });

  it("renders stored notifications and marks them read", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list"
        ? Promise.resolve([
            { id: "n1", kind: "mail", title: "Mail from Scout", body: "Ranked three threads", read: false, createdAt: 1 },
          ])
        : Promise.resolve(),
    );
    render(<Inbox open />);
    await waitFor(() => expect(screen.getByText(/Mail from Scout/)).toBeTruthy());
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("notifications_mark_read"));
  });
});