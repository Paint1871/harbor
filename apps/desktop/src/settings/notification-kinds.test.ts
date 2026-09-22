import { describe, expect, it } from "vitest";
import {
  NOTIFICATION_KIND_IDS,
  parseNotificationKinds,
  toggleNotificationKind,
} from "./notification-kinds";

describe("parseNotificationKinds", () => {
  it("defaults to every known kind when the setting is missing", () => {
    expect(parseNotificationKinds(undefined)).toEqual([...NOTIFICATION_KIND_IDS]);
    expect(parseNotificationKinds(null)).toEqual([...NOTIFICATION_KIND_IDS]);
    expect(parseNotificationKinds(true)).toEqual([...NOTIFICATION_KIND_IDS]);
    expect(parseNotificationKinds("mail")).toEqual([...NOTIFICATION_KIND_IDS]);
  });

  it("keeps known ids from an explicit list, in canonical order", () => {
    expect(parseNotificationKinds(["mail", "permission", "unknown", 1])).toEqual([
      "permission",
      "mail",
    ]);
    expect(parseNotificationKinds([])).toEqual([]);
  });
});

describe("toggleNotificationKind", () => {
  it("adds or removes one id and returns the updated allow-list", () => {
    expect(toggleNotificationKind(NOTIFICATION_KIND_IDS, "mail", false)).toEqual([
      "permission",
      "terminal-exit",
      "turn-finished",
      "turn-cancelled",
      "turn-refused",
      "turn-stopped",
      "routine",
    ]);
    expect(toggleNotificationKind(["permission"], "mail", true)).toEqual([
      "permission",
      "mail",
    ]);
  });
});
