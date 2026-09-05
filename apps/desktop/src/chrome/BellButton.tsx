import { Button } from "@harbor/ui/Button";

interface BellButtonProps {
  onClick: () => void;
}

export function BellButton({ onClick }: BellButtonProps) {
  return (
    <Button size="icon" variant="ghost" aria-label="Notifications" onClick={onClick}>
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M8 1.75c-2.2 0-4 1.7-4 3.9v1.7c0 .7-.3 1.4-.8 1.9L2.4 10.2c-.3.3-.1.8.3.8h10.6c.4 0 .6-.5.3-.8l-.8-.95A2.7 2.7 0 0 1 12 7.35V5.65c0-2.2-1.8-3.9-4-3.9Z"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
        <path d="M6.4 12.4a1.7 1.7 0 0 0 3.2 0" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      </svg>
    </Button>
  );
}
