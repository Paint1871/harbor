import { useEffect, useId, useState } from "react";

export interface MentionItem {
  id: string;
  label: string;
}

export interface MentionKeyEvent {
  key: string;
  preventDefault: () => void;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  isComposing?: boolean;
}

export function mentionQuery(value: string): string | null {
  const match = /(?:^|\s)@([^\s]*)$/.exec(value);
  return match ? (match[1] ?? "") : null;
}

function wrapIndex(index: number, length: number): number {
  if (length <= 0) return 0;
  return ((index % length) + length) % length;
}

/** Arrow/Tab/Enter/Escape while a mention list is open. `null` leaves the event to the composer. */
export function mentionKeydown(
  event: MentionKeyEvent,
  items: readonly MentionItem[],
  index: number,
): { index: number; pick?: string; cancel?: boolean } | null {
  if (!items.length || event.isComposing || event.metaKey || event.ctrlKey || event.altKey) return null;
  const current = wrapIndex(index, items.length);
  if (event.key === "ArrowDown") {
    event.preventDefault();
    return { index: wrapIndex(current + 1, items.length) };
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    return { index: wrapIndex(current - 1, items.length) };
  }
  if (event.key === "Enter" || event.key === "Tab") {
    const item = items[current];
    if (!item) return null;
    event.preventDefault();
    return { index: current, pick: item.id };
  }
  if (event.key === "Escape") {
    event.preventDefault();
    return { index: current, cancel: true };
  }
  return null;
}

function mentionTargetBlocksKeys(target: EventTarget | null): boolean {
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLTextAreaElement && target.getAttribute("aria-label") !== "Message") return true;
  return false;
}

export function MentionList({
  items,
  label,
  onPick,
  onCancel,
  activeIndex,
  onActiveIndexChange,
}: {
  items: MentionItem[];
  label: string;
  onPick: (id: string) => void;
  onCancel?: () => void;
  activeIndex?: number;
  onActiveIndexChange?: (index: number) => void;
}) {
  const reactId = useId();
  const [internalIndex, setInternalIndex] = useState(0);
  const count = items.length;
  const active = wrapIndex(activeIndex ?? internalIndex, count);

  useEffect(() => {
    if (!count) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (mentionTargetBlocksKeys(event.target)) return;
      const result = mentionKeydown(event, items, active);
      if (!result) return;
      if (activeIndex === undefined) setInternalIndex(result.index);
      onActiveIndexChange?.(result.index);
      if (result.pick) onPick(result.pick);
      else if (result.cancel) onCancel?.();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [active, activeIndex, count, items, onActiveIndexChange, onCancel, onPick]);

  if (!count) return null;

  const activeOptionId = `${reactId}-opt-${active}`;
  return (
    <ul
      className="harbor-tree"
      role="listbox"
      aria-label={label}
      aria-activedescendant={activeOptionId}
    >
      {items.map((item, index) => (
        <li
          key={item.id}
          id={`${reactId}-opt-${index}`}
          role="option"
          aria-selected={index === active}
        >
          <button type="button" onClick={() => onPick(item.id)}>
            {item.label}
          </button>
        </li>
      ))}
    </ul>
  );
}
