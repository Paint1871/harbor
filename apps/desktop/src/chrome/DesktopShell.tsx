import { useEffect, useMemo, useRef, useState } from "react";
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
import { settingsSet } from "../settings";
import { Dashboard } from "../destinations/Dashboard";
import { Plugins } from "../destinations/Plugins";
import { Routines } from "../destinations/Routines";
import { Skills } from "../destinations/Skills";
import { AppRail } from "./AppRail";
import { DestinationWorkspaceRail } from "./DestinationWorkspaceRail";

type SettingsPage = "general" | "notifications" | "voice" | "agents" | "account";

export interface DesktopShellProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  profileName?: string;
  onProfileNameChange?: (value: string) => void;
  reduceMotion?: boolean;
  onReduceMotionChange?: (value: boolean) => void;
  initialMode?: Mode;
  onShowWelcome?: () => void;
}

export function DesktopShell({
  theme,
  onThemeChange,
  profileName = "Local",
  onProfileNameChange,
  reduceMotion = false,
  onReduceMotionChange,
  initialMode = "agent",
  onShowWelcome,
}: DesktopShellProps) {
  const [mode, setMode] = useState<Mode>(initialMode);
  const [railOpen, setRailOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPage, setSettingsPage] = useState<SettingsPage>("general");
  const [inboxOpen, setInboxOpen] = useState(false);
  const [destination, setDestination] = useState<Destination>("mode");
  const tidyHandler = useRef<() => void>(() => undefined);
  const codePaneSelectHandler = useRef<(workspaceId: string, paneId: string) => void>(() => undefined);

  useEffect(() => {
    void settingsSet("last_mode", mode);
  }, [mode]);

  const chrome = useMemo(
    () => ({
      mode,
      theme,
      profileName,
      destination,
      setDestination,
      onModeChange: (next: Mode) => { setMode(next); setDestination("mode"); },
      onCodePaneSelect: (workspaceId: string, paneId: string) => {
        setMode("code");
        setDestination("mode");
        codePaneSelectHandler.current(workspaceId, paneId);
      },
      onThemeChange,
      onSettings: () => { setSettingsPage("general"); setSettingsOpen(true); },
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
          onTidy={() => tidyHandler.current()}
          onBellClick={() => setInboxOpen((open) => !open)}
          onOpenVoiceSettings={() => {
            setSettingsPage("voice");
            setSettingsOpen(true);
          }}
        />
        <Inbox open={inboxOpen} />
        {settingsOpen ? (
          <Settings
            theme={theme}
            onThemeChange={onThemeChange}
            initialPage={settingsPage}
            profileName={profileName}
            onProfileNameChange={onProfileNameChange}
            reduceMotion={reduceMotion}
            onReduceMotionChange={onReduceMotionChange}
            onClose={() => setSettingsOpen(false)}
            onShowWelcome={() => onShowWelcome?.()}
          />
        ) : null}
        <div className="harbor-body">
          <div className="harbor-stage" data-destination={destination} data-rail={railOpen}>
            {destination !== "mode" && railOpen ? (
              <AppRail className="harbor-destination-rail">
                <DestinationWorkspaceRail />
              </AppRail>
            ) : null}
            {destination !== "mode" ? (
              <div className="harbor-destination-view">
                {destination === "dashboard" ? <Dashboard /> : null}
                {destination === "plugins" ? <Plugins /> : null}
                {destination === "routines" ? <Routines /> : null}
                {destination === "skills" ? <Skills /> : null}
              </div>
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
                <CodeMode
                  railOpen={railOpen}
                  onPaneSelectRegister={(handler) => { codePaneSelectHandler.current = handler; }}
                />
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
