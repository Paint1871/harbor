import type { Theme } from "@harbor/ui/theme";
import { Footer } from "./Footer";

export interface RailProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onSettings: () => void;
}

export function Rail({ theme, onThemeChange, onSettings }: RailProps) {
  return (
    <aside className="harbor-rail" aria-label="Sidebar">
      <div className="harbor-rail-body" />
      <Footer theme={theme} onThemeChange={onThemeChange} onSettings={onSettings} />
    </aside>
  );
}
