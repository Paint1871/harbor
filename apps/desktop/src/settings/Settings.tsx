import { useState } from "react";
import { Button } from "@harbor/ui/Button";
import { Segmented } from "@harbor/ui/Segmented";
import type { Theme } from "@harbor/ui/theme";
import { settingsSet } from "./api";

const PAGES = [
  { value: "general", label: "General" },
  { value: "notifications", label: "Notifications" },
  { value: "voice", label: "Voice" },
  { value: "agents", label: "Agents" },
  { value: "account", label: "Account" },
] as const;

type Page = (typeof PAGES)[number]["value"];

export interface SettingsProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onClose: () => void;
  onShowWelcome: () => void;
}

export function Settings({ theme, onThemeChange, onClose, onShowWelcome }: SettingsProps) {
  const [page, setPage] = useState<Page>("general");
  return (
    <div className="harbor-settings" role="dialog" aria-label="Settings">
      <header>
        <h2>Settings</h2>
        <Button onClick={onClose}>Close</Button>
      </header>
      <div className="harbor-settings-body">
        <nav>
          {PAGES.map((item) => (
            <Button key={item.value} variant={page === item.value ? "primary" : "ghost"} onClick={() => setPage(item.value)}>
              {item.label}
            </Button>
          ))}
        </nav>
        <section>
          {page === "general" ? (
            <>
              <h3>General</h3>
              <p>Appearance, zoom, startup mode, English UI, default shell, Recheck engines, Reduce Motion.</p>
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
                }}
              />
            </>
          ) : null}
          {page === "notifications" ? (
            <>
              <h3>Notifications</h3>
              <p>Event types: prompt, permission, question, complete, fail. No transcript text.</p>
            </>
          ) : null}
          {page === "voice" ? (
            <>
              <h3>Voice</h3>
              <p>Dictation is on-device. Cloud STT only if you paste your own URL. Voice copilot is a later release.</p>
            </>
          ) : null}
          {page === "agents" ? (
            <>
              <h3>Agents</h3>
              <p>App-wide pause for agent mail, default engine, memory defaults.</p>
            </>
          ) : null}
          {page === "account" ? (
            <>
              <h3>Account</h3>
              <p>Free · local. No cloud login.</p>
              <Button
                onClick={() => {
                  void settingsSet("onboarded_local", false);
                  onShowWelcome();
                }}
              >
                Show welcome again
              </Button>
            </>
          ) : null}
        </section>
      </div>
    </div>
  );
}
