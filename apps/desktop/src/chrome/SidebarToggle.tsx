import { Button } from "@harbor/ui/Button";

interface SidebarToggleProps {
  open: boolean;
  onToggle: () => void;
}

export function SidebarToggle({ open, onToggle }: SidebarToggleProps) {
  return (
    <Button
      size="icon"
      variant="ghost"
      aria-label={open ? "Hide sidebar" : "Show sidebar"}
      aria-pressed={open}
      onClick={onToggle}
    >
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" />
        <path d="M6 2.5v11" stroke="currentColor" strokeWidth="1.4" />
      </svg>
    </Button>
  );
}
