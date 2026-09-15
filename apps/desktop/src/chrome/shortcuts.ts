/** Global chrome shortcuts. Edit chords (cut/copy/paste) are never matched. */

export type Shortcut = "escape" | "settings" | "new-thread" | "new-chat-thread";

export interface ShortcutEvent {
  key: string;
  code?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey?: boolean;
  repeat?: boolean;
  isComposing?: boolean;
}

function accel(event: ShortcutEvent): boolean {
  return event.metaKey || event.ctrlKey;
}

function matches(event: ShortcutEvent, code: string, key: string): boolean {
  return event.code === code || event.key === key || event.key.toLowerCase() === key;
}

/** Read a keydown. `null` means Harbor should leave the event to the focused control. */
export function interpretShortcut(event: ShortcutEvent): Shortcut | null {
  if (event.isComposing || event.repeat) return null;
  if (event.key === "Escape") return "escape";
  if (!accel(event) || event.shiftKey) return null;
  if (matches(event, "Comma", ",") && !event.altKey) return "settings";
  if (matches(event, "KeyN", "n") && !event.altKey) return "new-thread";
  if (matches(event, "KeyT", "t") && event.altKey) return "new-chat-thread";
  return null;
}
