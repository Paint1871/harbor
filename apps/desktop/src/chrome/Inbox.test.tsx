// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Inbox } from "./Inbox";
import { ChromeProvider, type ChromeValue } from "./chrome-context";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

const chrome = (over: Partial<ChromeValue> = {}): ChromeValue => ({
  mode: "agent",
  theme: "black",
  profileName: "Builder",
  destination: "mode",
  setDestination: () => undefined,
  onModeChange: () => undefined,
  onOpenSession: () => undefined,
  onThemeChange: () => undefined,
  onSettings: () => undefined,
  onTidy: () => undefined,
  registerTidy: () => undefined,
  ...over,
});

const row = {
  id: "n1",
  kind: "terminal-exit",
  title: "Codex stopped",
  body: "The terminal exited.",
  read: false,
  createdAt: Math.floor(Date.now() / 1000),
  mode: "code",
  workspaceId: "ws-2",
  paneId: "term-7",
  sessionRef: null,
};

describe("Inbox", () => {
  afterEach(() => cleanup());
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
    mocks.listen.mockResolvedValue(() => undefined);
  });

  it("shows an empty state instead of invented events", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([]) : Promise.resolve(),
    );
    render(<Inbox open />);
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("notifications_list"));
    expect(screen.getByText(/Nothing yet/)).toBeTruthy();
  });

  it("marks rows read when opened and jumps to the pane the event came from", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([row]) : Promise.resolve(),
    );
    const onOpenSession = vi.fn();
    const onClose = vi.fn();
    render(
      <ChromeProvider value={chrome({ onOpenSession })}>
        <Inbox open onClose={onClose} />
      </ChromeProvider>,
    );

    await waitFor(() => expect(screen.getByText("Codex stopped")).toBeTruthy());
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("notifications_mark_read"));

    fireEvent.click(screen.getByRole("button", { name: /Codex stopped/ }));
    expect(onOpenSession).toHaveBeenCalledWith({
      mode: "code",
      sessionRef: null,
      workspaceId: "ws-2",
      paneId: "term-7",
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("opens the chat thread a notification came from", async () => {
    const chatRow = {
      ...row,
      id: "n2",
      kind: "turn-finished",
      title: "Claude finished",
      body: "The turn completed.",
      mode: "chat",
      workspaceId: "ws-1",
      paneId: null,
      sessionRef: "thread-9",
    };
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([chatRow]) : Promise.resolve(),
    );
    const onOpenSession = vi.fn();
    const onClose = vi.fn();
    render(
      <ChromeProvider value={chrome({ onOpenSession })}>
        <Inbox open onClose={onClose} />
      </ChromeProvider>,
    );

    await waitFor(() => expect(screen.getByText("Claude finished")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: /Claude finished/ }));
    expect(onOpenSession).toHaveBeenCalledWith({
      mode: "chat",
      sessionRef: "thread-9",
      workspaceId: "ws-1",
      paneId: null,
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("filters rows by kind", async () => {
    const mail = {
      ...row,
      id: "n-mail",
      kind: "mail",
      title: "Mail from Ada",
      body: "Please take the deploy.",
      mode: "agent",
      workspaceId: null,
      paneId: null,
      sessionRef: "chat-a",
    };
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([mail, row]) : Promise.resolve(),
    );
    render(
      <ChromeProvider value={chrome()}>
        <Inbox open />
      </ChromeProvider>,
    );

    await waitFor(() => expect(screen.getByText("Mail from Ada")).toBeTruthy());
    expect(screen.getByText("Codex stopped")).toBeTruthy();

    const filters = screen.getByRole("group", { name: "Filter events" });
    fireEvent.click(within(filters).getByRole("button", { name: "Mail" }));
    expect(screen.getByText("Mail from Ada")).toBeTruthy();
    expect(screen.queryByText("Codex stopped")).toBeNull();

    fireEvent.click(within(filters).getByRole("button", { name: "All" }));
    expect(screen.getByText("Codex stopped")).toBeTruthy();
  });

  it("refreshes when the host pushes a new event", async () => {
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([]) : Promise.resolve(),
    );
    render(<Inbox open />);
    await waitFor(() => expect(mocks.listen).toHaveBeenCalledWith("notification", expect.any(Function)));

    const reload = mocks.listen.mock.calls[0][1] as () => void;
    mocks.invoke.mockImplementation((command: string) =>
      command === "notifications_list" ? Promise.resolve([row]) : Promise.resolve(),
    );
    reload();
    await waitFor(() => expect(screen.getByText("Codex stopped")).toBeTruthy());
  });

  it("stays out of the tab order while closed", () => {
    mocks.invoke.mockImplementation(() => Promise.resolve([]));
    const { container } = render(<Inbox open={false} />);
    expect(container.querySelector(".harbor-inbox")?.getAttribute("data-open")).toBe("false");
    expect(container.querySelector(".harbor-inbox")?.hasAttribute("inert")).toBe(true);
  });
});
