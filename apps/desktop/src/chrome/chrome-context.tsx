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
  onModeChange: (mode: Mode) => void;
  onCodePaneSelect?: (workspaceId: string, paneId: string) => void;
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

/** Context-aware screens can also be rendered in isolation by the UI tests. */
export function useOptionalChrome(): ChromeValue | null {
  return useContext(ChromeContext);
}
