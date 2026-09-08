import { useEffect, useId, useMemo, useRef, useState } from "react";

export interface MenuItem {
  id: string;
  name: string;
  description?: string | null;
  group?: string | null;
}

export interface MenuPillProps {
  /** What the control is, shown as the menu's heading. */
  label: string;
  /** What is chosen, shown on the pill itself. */
  value: string;
  /** Put the label on the pill too, where the value alone says nothing ("Off"). */
  showLabel?: boolean;
  items: MenuItem[];
  current: string | null;
  onSelect: (id: string) => void;
  disabled?: boolean;
  busy?: boolean;
  onOpen?: () => void;
  emptyNote?: string;
  footer?: React.ReactNode;
  tone?: "default" | "quiet";
}

/** A list this long stops being scannable without a filter. */
const SEARCH_THRESHOLD = 10;

export function MenuPill({ label, value, showLabel = false, items, current, onSelect, disabled = false, busy = false, onOpen, emptyNote, footer, tone = "default" }: MenuPillProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const searchable = items.length > SEARCH_THRESHOLD;

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return items;
    return items.filter((item) => `${item.name} ${item.id} ${item.group ?? ""}`.toLowerCase().includes(needle));
  }, [items, query]);

  const groups = useMemo(() => {
    const order: string[] = [];
    const byGroup = new Map<string, MenuItem[]>();
    for (const item of visible) {
      const key = item.group ?? "";
      if (!byGroup.has(key)) {
        byGroup.set(key, []);
        order.push(key);
      }
      byGroup.get(key)!.push(item);
    }
    return order.map((key) => [key, byGroup.get(key)!] as const);
  }, [visible]);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="harbor-menu-pill" ref={root}>
      <button
        type="button"
        className="harbor-pill-trigger"
        data-tone={tone}
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        aria-label={`${label}: ${value}`}
        onClick={() => {
          setOpen((was) => {
            if (!was) {
              setQuery("");
              onOpen?.();
            }
            return !was;
          });
        }}
      >
        {showLabel ? <span className="harbor-pill-trigger-label">{label}</span> : null}
        <span>{value}</span>
        <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2.5 4 5 6.5 7.5 4" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      {open ? (
        <div className="harbor-menu harbor-pill-menu" id={menuId} role="menu" aria-label={label}>
          <p className="harbor-pill-menu-label">{label}</p>
          {searchable ? (
            <input
              className="harbor-pill-menu-search"
              aria-label={`Filter ${label}`}
              placeholder="Filter…"
              autoFocus
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          ) : null}
          <div className="harbor-pill-menu-scroll">
            {groups.map(([group, entries]) => (
              <div key={group || "_"}>
                {group ? <p className="harbor-pill-menu-group">{group}</p> : null}
                {entries.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    role="menuitemradio"
                    aria-checked={item.id === current}
                    onClick={() => {
                      setOpen(false);
                      if (item.id !== current) onSelect(item.id);
                    }}
                  >
                    <span className="harbor-pill-menu-name">
                      {item.name}
                      {item.description ? <small>{item.description}</small> : null}
                    </span>
                    {item.id === current ? <span className="harbor-pill-menu-check" aria-hidden="true">✓</span> : null}
                  </button>
                ))}
              </div>
            ))}
            {!visible.length ? (
              <p className="harbor-pill-menu-note">
                {busy ? "Asking the engine…" : query.trim() ? `Nothing matches “${query.trim()}”.` : emptyNote ?? "Nothing to choose here."}
              </p>
            ) : null}
          </div>
          {footer}
        </div>
      ) : null}
    </div>
  );
}
