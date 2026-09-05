import { createContext, useContext } from "react";
import type { Theme } from "@harbor/ui/theme";
import type { Mode } from "./ModeSwitch";

export type Destination = "mode" | "dashboard" | "plugins" | "routines" | "skills";

export interface ChromeValue {
  mode: Mode;
  theme: Theme;
  profileName: string;
  destination: Destination;
  setDestination: (destination: Destination) => void;
  onThemeChange: (theme: Theme) => void;
  onSettings: () => void;
  onTidy: () => void;
  registerTidy: (handler: () => void) => void;
}

const ChromeContext = createContext<ChromeValue | null>(null);

export const ChromeProvider = ChromeContext.Provider;

export function useChrome(): ChromeValue {
  const value = useContext(ChromeContext);
  if (!value) {
    throw new Error("useChrome must be used within DesktopShell");
  }
  return value;
}
