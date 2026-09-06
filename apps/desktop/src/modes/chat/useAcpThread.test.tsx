// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAcpThread } from "./useAcpThread";
import type { ChatMessage } from "@harbor/schema/commands";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => undefined }));
const message = (id: string, text: string): ChatMessage => ({ id, text, role: "user" });
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

beforeEach(() => { invoke.mockReset(); });
describe("thread transcript isolation", () => {
  it("never displays a late history response from a previously selected thread", async () => {
    const first = deferred<ChatMessage[]>();
    invoke.mockImplementation((_command, { id }) => id === "a" ? first.promise : Promise.resolve([message("b1", "Only B")]));
    const hook = renderHook(({ id }) => useAcpThread(id), { initialProps: { id: "a" } });
    hook.rerender({ id: "b" });
    await waitFor(() => expect(hook.result.current.lines[0]?.text).toBe("Only B"));
    await act(async () => first.resolve([message("a1", "Only A")]));
    expect(hook.result.current.lines.map((line) => line.text)).toEqual(["Only B"]);
    hook.unmount();
  });

  it("keeps an in-flight turn with its originating thread and prevents double send", async () => {
    const turn = deferred<void>();
    const persisted: Record<string, ChatMessage[]> = { a: [], b: [message("b1", "B history")] };
    invoke.mockImplementation((command, args) => command === "thread_history" ? Promise.resolve(persisted[args.id]) : turn.promise);
    const hook = renderHook(({ id }) => useAcpThread(id), { initialProps: { id: "a" } });
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    let sending!: Promise<boolean>;
    act(() => { sending = hook.result.current.send("A message"); void hook.result.current.send("duplicate"); });
    expect(invoke.mock.calls.filter(([command]) => command === "thread_send")).toHaveLength(1);
    hook.rerender({ id: "b" });
    await waitFor(() => expect(hook.result.current.lines[0]?.text).toBe("B history"));
    persisted.a = [message("a1", "A message"), { id: "a2", role: "assistant", text: "A reply" }];
    await act(async () => { turn.resolve(); await sending; });
    expect(hook.result.current.lines[0]?.text).toBe("B history");
    hook.rerender({ id: "a" });
    await waitFor(() => expect(hook.result.current.lines.map((line) => line.text)).toEqual(["A message", "A reply"]));
    expect(hook.result.current.sending).toBe(false);
    hook.unmount();
  });

  it("reports a rejected engine turn separately and reloads the stored user message", async () => {
    let stored: ChatMessage[] = [];
    invoke.mockImplementation((command) => {
      if (command === "thread_history") return Promise.resolve(stored);
      stored = [message("saved", "Keep this draft")];
      return Promise.reject(new Error("CLI is not on PATH"));
    });
    const hook = renderHook(() => useAcpThread("a"));
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    await act(async () => { expect(await hook.result.current.send("Keep this draft")).toBe(false); });
    expect(hook.result.current.error).toContain("CLI is not on PATH");
    expect(hook.result.current.lines).toEqual(stored);
    expect(hook.result.current.sending).toBe(false);
    hook.unmount();
  });

  it("does not submit a message without a thread", async () => {
    const hook = renderHook(() => useAcpThread(null));
    await act(async () => { expect(await hook.result.current.send("Keep me")).toBe(false); });
    expect(invoke).not.toHaveBeenCalled();
    hook.unmount();
  });

  it("cancels the active turn through thread_cancel", async () => {
    invoke.mockImplementation((command) => command === "thread_history" ? Promise.resolve([]) : Promise.resolve());
    const hook = renderHook(() => useAcpThread("a"));
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    await act(async () => { await hook.result.current.cancel(); });
    expect(invoke).toHaveBeenCalledWith("thread_cancel", { id: "a" });
    expect(hook.result.current.sending).toBe(false);
    hook.unmount();
  });
});
