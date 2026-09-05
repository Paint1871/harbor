import { useEffect, useState } from "react";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import type { Theme } from "@harbor/ui/theme";
import { DesktopShell } from "./chrome/DesktopShell";
import { settingsGet, settingsSet } from "./settings";
import { WelcomeScreen } from "./welcome/WelcomeScreen";

export function App() {
  const [theme, setTheme] = useState<Theme>("black");
  const [ready, setReady] = useState(false);
  const [onboarded, setOnboarded] = useState(false);
  const [profileName, setProfileName] = useState("Local");

  useEffect(() => {
    void settingsGet("onboarded_local").then((value) => {
      setOnboarded(value === true);
      setReady(true);
    });
    void settingsGet("local_profile_name").then((value) => {
      if (typeof value === "string" && value.trim()) setProfileName(value);
    });
  }, []);

  async function startLocal(name: string) {
    await settingsSet("onboarded_local", true);
    await settingsSet("local_profile_name", name);
    setProfileName(name);
    setOnboarded(true);
  }

  return (
    <ThemeProvider theme={theme}>
      {!ready ? null : !onboarded ? (
        <WelcomeScreen onStartLocal={startLocal} />
      ) : (
        <DesktopShell
          theme={theme}
          onThemeChange={setTheme}
          profileName={profileName}
          onShowWelcome={() => setOnboarded(false)}
        />
      )}
    </ThemeProvider>
  );
}
