/** Kinds `notify()` / Inbox actually record. Missing setting = all enabled. */
export const NOTIFICATION_KIND_IDS = [
  "permission",
  "mail",
  "terminal-exit",
  "turn-finished",
  "turn-cancelled",
  "turn-refused",
  "turn-stopped",
  "routine",
] as const;

export type NotificationKindId = (typeof NOTIFICATION_KIND_IDS)[number];

export const NOTIFICATION_KINDS: {
  id: NotificationKindId;
  label: string;
  description: string;
}[] = [
  { id: "permission", label: "Needs you", description: "Permission prompts waiting for an answer." },
  { id: "mail", label: "Mail", description: "Handoffs between local agents." },
  { id: "terminal-exit", label: "Terminal", description: "A terminal pane that exited." },
  { id: "turn-finished", label: "Finished", description: "A turn that completed while you were away." },
  { id: "turn-cancelled", label: "Cancelled", description: "A turn that was cancelled." },
  { id: "turn-refused", label: "Refused", description: "The engine refused the turn." },
  { id: "turn-stopped", label: "Stopped", description: "A turn that stopped for another reason." },
  { id: "routine", label: "Routines", description: "A scheduled routine that prepared a chat." },
];

function isKindId(value: unknown): value is NotificationKindId {
  return typeof value === "string" && (NOTIFICATION_KIND_IDS as readonly string[]).includes(value);
}

/** A missing or non-array value enables every known kind. An array is the allow-list. */
export function parseNotificationKinds(value: unknown): NotificationKindId[] {
  if (!Array.isArray(value)) return [...NOTIFICATION_KIND_IDS];
  const enabled = new Set(value.filter(isKindId));
  return NOTIFICATION_KIND_IDS.filter((id) => enabled.has(id));
}

export function toggleNotificationKind(
  enabled: readonly string[],
  kind: NotificationKindId,
  on: boolean,
): NotificationKindId[] {
  const next = new Set(enabled.filter(isKindId));
  if (on) next.add(kind);
  else next.delete(kind);
  return NOTIFICATION_KIND_IDS.filter((id) => next.has(id));
}
