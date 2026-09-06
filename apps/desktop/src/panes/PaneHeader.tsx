import { useEffect, useRef, useState, type ReactNode } from "react";

export interface PaneHeaderProps {
  title: string;
  live?: boolean;
  leading?: ReactNode;
  extra?: ReactNode;
  compact?: boolean;
  expanded?: boolean;
  onClear?: () => void;
  menuContent?: ReactNode;
  onExpand?: () => void;
  onSplit?: () => void;
  onClose?: () => void;
}

export function PaneHeader({ title, live = false, leading, extra, compact = false, expanded = false, onClear, menuContent, onExpand, onSplit, onClose }: PaneHeaderProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const close = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [menuOpen]);

  return (
    <header className={`harbor-pane-header${compact ? " harbor-pane-header-compact" : ""}`}>
      {!compact ? <span className="harbor-live-dot" data-on={live} /> : null}
      {leading ? <span className="harbor-pane-leading">{leading}</span> : null}
      {!compact ? <span className="harbor-pane-title">{title}</span> : null}
      <div className="harbor-pane-actions">
        {extra}
        <div className="harbor-pane-more" ref={menuRef}>
          <button
            type="button"
            aria-label={`More ${title}`}
            title={`More ${title}`}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onClick={() => setMenuOpen((open) => !open)}
          >
            <span aria-hidden="true">…</span>
          </button>
          {menuOpen ? (
            <div className="harbor-pane-menu" role="menu">
              {menuContent ? <div className="harbor-pane-menu-content">{menuContent}</div> : null}
              {onClear ? (
                <button type="button" role="menuitem" onClick={() => { onClear(); setMenuOpen(false); }}>
                  Clear terminal
                </button>
              ) : null}
              {onExpand ? (
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    onExpand();
                    setMenuOpen(false);
                  }}
                >
                  {expanded ? "Restore pane" : "Focus pane"}
                </button>
              ) : null}
              {onClose ? (
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    onClose();
                    setMenuOpen(false);
                  }}
                >
                  Close pane
                </button>
              ) : null}
            </div>
          ) : null}
        </div>
        {onExpand ? (
          <button
            type="button"
            aria-label={expanded ? `Restore ${title}` : `Expand ${title}`}
            title={expanded ? `Restore ${title}` : `Expand ${title}`}
            onClick={onExpand}
          >
            <svg width="13" height="13" viewBox="0 0 16 16" aria-hidden="true">
              {expanded ? (
                <path d="M6 2.5H2.5V6M10 13.5h3.5V10M2.5 2.5 7 7M13.5 13.5 9 9" fill="none" stroke="currentColor" strokeWidth="1.15" strokeLinecap="round" strokeLinejoin="round" />
              ) : (
                <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10M2.5 2.5 6 6M13.5 2.5 10 6M13.5 13.5 10 10M2.5 13.5 6 10" fill="none" stroke="currentColor" strokeWidth="1.15" strokeLinecap="round" strokeLinejoin="round" />
              )}
            </svg>
          </button>
        ) : null}
        <button type="button" aria-label={`Split ${title}`} title={`Split ${title}`} onClick={onSplit} disabled={!onSplit}>
          <span aria-hidden="true">+</span>
        </button>
        <button type="button" aria-label={`Close ${title}`} title={`Close ${title}`} onClick={onClose} disabled={!onClose}>
          <span aria-hidden="true">×</span>
        </button>
      </div>
    </header>
  );
}
