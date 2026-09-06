import { useEffect, useState } from "react";
import { ThemeProvider } from "@harbor/ui/ThemeProvider";
import type { Theme } from "@harbor/ui/theme";
import { DesktopShell } from "./chrome/DesktopShell";
import { settingsGet, settingsSet } from "./settings";
import { WelcomeScreen } from "./welcome/WelcomeScreen";

type StartupMode = "last" | "welcome" | "agent";
type HarborMode = "agent" | "code" | "chat";

export function App() {
  const [theme, setTheme] = useState<Theme>("black");
  const [ready, setReady] = useState(false);
  const [onboarded, setOnboarded] = useState(false);
  const [profileName, setProfileName] = useState("Local");
  const [reduceMotion, setReduceMotion] = useState(false);
  const [initialMode, setInitialMode] = useState<HarborMode>("code");

  useEffect(() => {
    void Promise.all([
      settingsGet("onboarded_local"),
      settingsGet("local_profile_name"),
      settingsGet("appearance"),
      settingsGet("reduce_motion"),
      settingsGet("startup_mode"),
      settingsGet("last_mode"),
    ]).then(([onboardedValue, name, appearance, reduceMotionValue, startupValue, lastModeValue]) => {
      setOnboarded(onboardedValue === true);
      if (typeof name === "string" && name.trim()) setProfileName(name);
      if (appearance === "black" || appearance === "light") setTheme(appearance);
      if (reduceMotionValue === true) setReduceMotion(true);
      const startup: StartupMode = startupValue === "welcome" || startupValue === "agent" || startupValue === "last" ? startupValue : "last";
      if (startup === "welcome") setOnboarded(false);
      const lastMode: HarborMode = lastModeValue === "agent" || lastModeValue === "code" || lastModeValue === "chat" ? lastModeValue : "code";
      setInitialMode(startup === "agent" ? "agent" : lastMode);
      setReady(true);
    });
  }, []);

  async function startLocal(name: string) {
    await settingsSet("onboarded_local", true);
    await settingsSet("local_profile_name", name);
    setProfileName(name);
    setOnboarded(true);
  }

  return (
    <ThemeProvider theme={theme} reduceMotion={reduceMotion}>
      {!ready ? null : !onboarded ? (
        <WelcomeScreen onStartLocal={startLocal} />
      ) : (
        <DesktopShell
          theme={theme}
          onThemeChange={setTheme}
          initialMode={initialMode}
          profileName={profileName}
          onProfileNameChange={setProfileName}
          reduceMotion={reduceMotion}
          onReduceMotionChange={setReduceMotion}
          onShowWelcome={() => setOnboarded(false)}
        />
      )}
    </ThemeProvider>
  );
}
