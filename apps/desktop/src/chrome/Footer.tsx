import { Button } from "@harbor/ui/Button";
import { useChrome } from "./chrome-context";

export function Footer() {
  const { profileName, onSettings } = useChrome();
  const initial = profileName.trim().charAt(0).toUpperCase() || "L";
  return (
    <footer className="harbor-footer">
      <div className="harbor-profile">
        <span className="harbor-profile-mark" aria-hidden="true">
          {initial}
        </span>
        <span className="harbor-profile-name">{profileName || "Local"}</span>
        <Button size="icon" variant="ghost" aria-label="Settings" onClick={onSettings}>
          <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.4" />
            <path
              d="M8 1.6v1.5M8 12.9v1.5M1.6 8h1.5M12.9 8h1.5M3.3 3.3l1.1 1.1M11.6 11.6l1.1 1.1M12.7 3.3l-1.1 1.1M4.4 11.6l-1.1 1.1"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.4"
              strokeLinecap="round"
            />
          </svg>
        </Button>
      </div>
    </footer>
  );
}
