import { useEffect, useState } from "react";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import type { Theme } from "@harbor/ui/theme";
import { Inbox } from "./chrome/Inbox";
import { Rail } from "./chrome/Rail";
import { TitleBar } from "./chrome/TitleBar";
import { Settings } from "./settings/Settings";
import type { Mode } from "./chrome/ModeSwitch";
import { AgentMode } from "./modes/AgentMode";
import { ChatMode } from "./modes/ChatMode";
import { CodeMode } from "./modes/CodeMode";
import { hostPlatform } from "./platform";
import { settingsGet, settingsSet } from "./settings";
import { WelcomeScreen } from "./welcome/WelcomeScreen";

export function App() {
  const [mode, setMode] = useState<Mode>("agent");
  const [railOpen, setRailOpen] = useState(true);
  const [theme, setTheme] = useState<Theme>("black");
  const [voiceNotice, setVoiceNotice] = useState(false);
  const [ready, setReady] = useState(false);
  const [onboarded, setOnboarded] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [inboxOpen, setInboxOpen] = useState(false);

  useEffect(() => {
    void settingsGet("onboarded_local").then((value) => {
      setOnboarded(value === true);
      setReady(true);
    });
  }, []);

  async function startLocal(profileName: string) {
    await settingsSet("onboarded_local", true);
    await settingsSet("local_profile_name", profileName);
    setOnboarded(true);
    setMode("agent");
    setRailOpen(true);
  }

  return (
    <ThemeProvider theme={theme}>
      {!ready ? null : !onboarded ? (
        <WelcomeScreen onStartLocal={startLocal} />
      ) : (
        <div className="harbor-app" data-platform={hostPlatform()}>
          <TitleBar
            mode={mode}
            onModeChange={setMode}
            railOpen={railOpen}
            onToggleRail={() => setRailOpen((open) => !open)}
            onOrbClick={() => setVoiceNotice(true)}
            onTidy={() => undefined}
            onBellClick={() => setInboxOpen((open) => !open)}
          />
          <Inbox open={inboxOpen} />
          {settingsOpen ? (
            <Settings
              theme={theme}
              onThemeChange={setTheme}
              onClose={() => setSettingsOpen(false)}
              onShowWelcome={() => {
                setSettingsOpen(false);
                setOnboarded(false);
              }}
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
            {railOpen ? <Rail theme={theme} onThemeChange={setTheme} onSettings={() => setSettingsOpen(true)} /> : null}
            <div className="harbor-modes">
              <section className="harbor-mode" data-active={mode === "agent"} aria-hidden={mode !== "agent"} aria-label="Agent">
                <AgentMode />
              </section>
              <section className="harbor-mode" data-active={mode === "code"} aria-hidden={mode !== "code"} aria-label="Code">
                <CodeMode />
              </section>
              <section className="harbor-mode" data-active={mode === "chat"} aria-hidden={mode !== "chat"} aria-label="Chat">
                <ChatMode />
              </section>
            </div>
          </div>
        </div>
      )}
    </ThemeProvider>
  );
}
