import { useState } from "react";
import { Button } from "@harbor/ui/Button";
import { Segmented } from "@harbor/ui/Segmented";
import { settingsSet } from "../settings";
import { useChrome } from "./chrome-context";

export function Footer() {
  const { profileName, theme, onThemeChange, onSettings } = useChrome();
  const [appearanceOpen, setAppearanceOpen] = useState(false);
  const initial = profileName.trim().charAt(0).toUpperCase() || "L";
  return (
    <footer className="harbor-footer">
      <div className="harbor-footer-account">
        <div className="harbor-profile">
          <span className="harbor-profile-mark" aria-hidden="true">
            {initial}
          </span>
          <span className="harbor-profile-copy">
            <span className="harbor-profile-name">{profileName.trim() || "Local"}</span>
            <span className="harbor-profile-plan">Free · local</span>
          </span>
        </div>
        <div className="harbor-footer-account-actions">
          <div className="harbor-footer-appearance">
            <Button
              size="icon"
              variant="ghost"
              aria-label="Appearance"
              title="Appearance"
              aria-expanded={appearanceOpen}
              onClick={() => setAppearanceOpen((open) => !open)}
            >
              <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true">
                <path d="M8 2.25a5.75 5.75 0 1 0 5.75 5.75A4.1 4.1 0 0 1 8 2.25Z" fill="none" stroke="currentColor" strokeWidth="1.35" strokeLinejoin="round" />
              </svg>
            </Button>
            {appearanceOpen ? (
              <div className="harbor-footer-appearance-menu">
                <Segmented
                  label="Appearance"
                  value={theme}
                  options={[
                    { value: "black", label: "Black" },
                    { value: "light", label: "Light" },
                  ]}
                  onValueChange={(value) => {
                    onThemeChange(value);
                    void settingsSet("appearance", value);
                    setAppearanceOpen(false);
                  }}
                />
              </div>
            ) : null}
          </div>
          <Button size="icon" variant="ghost" aria-label="Settings" title="Settings" onClick={onSettings}>
            <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true">
              <path d="m8 2 .55 1.35a4.96 4.96 0 0 1 1.17.49L11 3.3l1.7 1.7-.54 1.28c.2.36.36.75.48 1.17L14 8l-1.36.55a4.96 4.96 0 0 1-.49 1.17l.55 1.28-1.7 1.7-1.28-.54a4.96 4.96 0 0 1-1.17.48L8 14l-.55-1.36a4.96 4.96 0 0 1-1.17-.49L5 12.7 3.3 11l.54-1.28a4.96 4.96 0 0 1-.48-1.17L2 8l1.36-.55a4.96 4.96 0 0 1 .49-1.17L3.3 5 5 3.3l1.28.54a4.96 4.96 0 0 1 1.17-.48L8 2Z" fill="none" stroke="currentColor" strokeWidth="1.05" strokeLinejoin="round" />
              <circle cx="8" cy="8" r="1.65" fill="none" stroke="currentColor" strokeWidth="1.05" />
            </svg>
          </Button>
        </div>
      </div>
    </footer>
  );
}
