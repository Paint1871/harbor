import { createContext, useContext, useLayoutEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { applyTheme } from "./theme";
import type { Theme } from "./theme";

const motionQuery = "(prefers-reduced-motion: reduce)";

function subscribeMotion(onChange: () => void) {
  const media = window.matchMedia(motionQuery);
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

function getMotionSnapshot() {
  return window.matchMedia(motionQuery).matches;
}

function getServerMotionSnapshot() {
  return true;
}

interface Appearance {
  theme: Theme;
  reducedMotion: boolean;
}

const ThemeContext = createContext<Appearance | null>(null);

export interface ThemeProviderProps {
  children: ReactNode;
  theme?: Theme;
  /** An app preference can reduce motion; it never overrides the OS accessibility setting. */
  reduceMotion?: boolean;
}

/** One root provider. The host owns settings persistence; this package performs no I/O. */
export function ThemeProvider({ children, theme = "black", reduceMotion = false }: ThemeProviderProps) {
  const systemReducedMotion = useSyncExternalStore(
    subscribeMotion, getMotionSnapshot, getServerMotionSnapshot,
  );
  const reducedMotion = reduceMotion || systemReducedMotion;

  useLayoutEffect(() => {
    const root = document.documentElement;
    const previousTheme = root.dataset.theme;
    const previousScheme = root.style.colorScheme;
    const previousMotion = root.dataset.reduceMotion;
    applyTheme(theme, root);
    root.dataset.reduceMotion = String(reducedMotion);
    return () => {
      if (previousTheme === undefined) delete root.dataset.theme;
      else root.dataset.theme = previousTheme;
      root.style.colorScheme = previousScheme;
      if (previousMotion === undefined) delete root.dataset.reduceMotion;
      else root.dataset.reduceMotion = previousMotion;
    };
  }, [theme, reducedMotion]);

  return <ThemeContext value={{ theme, reducedMotion }}>{children}</ThemeContext>;
}

export function useTheme(): Appearance {
  const appearance = useContext(ThemeContext);
  if (!appearance) throw new Error("useTheme requires a ThemeProvider");
  return appearance;
}
