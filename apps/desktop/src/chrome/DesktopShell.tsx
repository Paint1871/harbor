import { useMemo, useRef, useState } from "react";
import type { Theme } from "@harbor/ui/theme";
import { Inbox } from "./Inbox";
import { TitleBar } from "./TitleBar";
import { ChromeProvider, type Destination } from "./chrome-context";
import type { Mode } from "./ModeSwitch";
import { AgentMode } from "../modes/AgentMode";
import { ChatMode } from "../modes/ChatMode";
import { CodeMode } from "../modes/CodeMode";
import { hostPlatform } from "../platform";
import { Settings } from "../settings/Settings";
import { Dashboard } from "../destinations/Dashboard";
import { Plugins } from "../destinations/Plugins";

export interface DesktopShellProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  profileName?: string;
  initialMode?: Mode;
  onShowWelcome?: () => void;
}

export function DesktopShell({
  theme,
  onThemeChange,
  profileName = "Local",
  initialMode = "agent",
  onShowWelcome,
}: DesktopShellProps) {
  const [mode, setMode] = useState<Mode>(initialMode);
  const [railOpen, setRailOpen] = useState(true);
  const [voiceNotice, setVoiceNotice] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [inboxOpen, setInboxOpen] = useState(false);
  const [destination, setDestination] = useState<Destination>("mode");
  const tidyHandler = useRef<() => void>(() => undefined);

  const chrome = useMemo(
    () => ({
      mode,
      theme,
      profileName,
      destination,
      setDestination,
      onThemeChange,
      onSettings: () => setSettingsOpen(true),
      onTidy: () => tidyHandler.current(),
      registerTidy: (handler: () => void) => {
        tidyHandler.current = handler;
      },
    }),
    [destination, mode, onThemeChange, profileName, theme],
  );

  return (
    <ChromeProvider value={chrome}>
      <div className="harbor-app" data-platform={hostPlatform()}>
        <TitleBar
          mode={mode}
          onModeChange={(next) => {
            setMode(next);
            setDestination("mode");
          }}
          railOpen={railOpen}
          onToggleRail={() => setRailOpen((open) => !open)}
          onOrbClick={() => setVoiceNotice(true)}
          onTidy={() => tidyHandler.current()}
          onBellClick={() => setInboxOpen((open) => !open)}
        />
        <Inbox open={inboxOpen} />
        {settingsOpen ? (
          <Settings
            theme={theme}
            onThemeChange={onThemeChange}
            onClose={() => setSettingsOpen(false)}
            onShowWelcome={() => onShowWelcome?.()}
          />
        ) : null}
        {voiceNotice ? (
          <div className="harbor-voice-notice" role="status">
            <p>Voice is off until a later release</p>
            <button type="button" onClick={() => setVoiceNotice(false)}>
              Settings → Voice
            </button>
          </div>
        ) : null}
        <div className="harbor-body">
          <div className="harbor-stage" data-destination={destination} data-rail={railOpen}>
            {destination === "dashboard" ? <Dashboard /> : null}
            {destination === "plugins" ? <Plugins /> : null}
            {destination === "routines" ? (
              <section className="harbor-destination-page" aria-label="Routines">
                <h2>Routines</h2>
                <p className="harbor-muted">Routines ship after 0.1.0.</p>
              </section>
            ) : null}
            {destination === "skills" ? (
              <section className="harbor-destination-page" aria-label="Skills">
                <h2>Skills</h2>
                <p className="harbor-muted">Skills ship after 0.1.0.</p>
              </section>
            ) : null}
            <div className="harbor-modes" data-rail={railOpen}>
              <section
                className="harbor-mode"
                data-active={mode === "agent"}
                aria-hidden={mode !== "agent"}
                aria-label="Agent"
              >
                <AgentMode railOpen={railOpen} />
              </section>
              <section
                className="harbor-mode"
                data-active={mode === "code"}
                aria-hidden={mode !== "code"}
                aria-label="Code"
              >
                <CodeMode railOpen={railOpen} />
              </section>
              <section
                className="harbor-mode"
                data-active={mode === "chat"}
                aria-hidden={mode !== "chat"}
                aria-label="Chat"
              >
                <ChatMode railOpen={railOpen} />
              </section>
            </div>
          </div>
        </div>
      </div>
    </ChromeProvider>
  );
}
