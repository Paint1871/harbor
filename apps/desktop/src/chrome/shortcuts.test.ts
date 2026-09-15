import { describe, expect, it } from "vitest";
import { interpretShortcut, type ShortcutEvent } from "./shortcuts";

function event(partial: Partial<ShortcutEvent> & Pick<ShortcutEvent, "key">): ShortcutEvent {
  return {
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...partial,
  };
}

describe("interpretShortcut", () => {
  it("opens settings on Cmd/Ctrl+,", () => {
    expect(interpretShortcut(event({ key: ",", code: "Comma", metaKey: true }))).toBe("settings");
    expect(interpretShortcut(event({ key: ",", code: "Comma", ctrlKey: true }))).toBe("settings");
  });

  it("creates a mode-dependent thread on Cmd/Ctrl+N", () => {
    expect(interpretShortcut(event({ key: "n", code: "KeyN", metaKey: true }))).toBe("new-thread");
    expect(interpretShortcut(event({ key: "N", code: "KeyN", ctrlKey: true }))).toBe("new-thread");
  });

  it("opens a Chat thread from Code on Opt+Cmd+T / Alt+Ctrl+T", () => {
    expect(interpretShortcut(event({ key: "t", code: "KeyT", metaKey: true, altKey: true }))).toBe("new-chat-thread");
    expect(interpretShortcut(event({ key: "T", code: "KeyT", ctrlKey: true, altKey: true }))).toBe("new-chat-thread");
    expect(interpretShortcut(event({ key: "†", code: "KeyT", metaKey: true, altKey: true }))).toBe("new-chat-thread");
  });

  it("closes overlays on Escape", () => {
    expect(interpretShortcut(event({ key: "Escape" }))).toBe("escape");
  });

  it("does not hijack edit chords, Enter, or shifted/option variants", () => {
    expect(interpretShortcut(event({ key: "c", code: "KeyC", metaKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "v", code: "KeyV", metaKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "x", code: "KeyX", ctrlKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "Enter" }))).toBeNull();
    expect(interpretShortcut(event({ key: "n", code: "KeyN", metaKey: true, shiftKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "n", code: "KeyN", metaKey: true, altKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "t", code: "KeyT", metaKey: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "t", code: "KeyT", altKey: true }))).toBeNull();
  });

  it("ignores repeats and IME composition", () => {
    expect(interpretShortcut(event({ key: "n", code: "KeyN", metaKey: true, repeat: true }))).toBeNull();
    expect(interpretShortcut(event({ key: ",", code: "Comma", metaKey: true, isComposing: true }))).toBeNull();
    expect(interpretShortcut(event({ key: "Escape", repeat: true }))).toBeNull();
  });
});
