import type { ReactNode } from "react";

export interface PaneHeaderProps {
  title: string;
  live?: boolean;
  extra?: ReactNode;
  onSplit?: () => void;
  onClose?: () => void;
}

export function PaneHeader({ title, live = false, extra, onSplit, onClose }: PaneHeaderProps) {
  return (
    <header className="harbor-pane-header">
      <span className="harbor-live-dot" data-on={live} />
      <span className="harbor-pane-title">{title}</span>
      <span className="harbor-pane-actions">
        {extra}
        <button type="button" aria-label={`Split ${title}`} onClick={onSplit}>
          Split
        </button>
        <button type="button" aria-label={`Close ${title}`} onClick={onClose}>
          Close
        </button>
      </span>
    </header>
  );
}
