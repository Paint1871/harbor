import { Button } from "@harbor/ui/Button";
import { Pill } from "@harbor/ui/Pill";
import { Segmented } from "@harbor/ui/Segmented";
import type { Theme } from "@harbor/ui/theme";

const APPEARANCE = [
  { value: "black", label: "Black" },
  { value: "light", label: "Light" },
] as const;

export interface FooterProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  onSettings: () => void;
}

export function Footer({ theme, onThemeChange, onSettings }: FooterProps) {
  return (
    <footer className="harbor-footer">
      <Segmented label="Appearance" value={theme} options={APPEARANCE} onValueChange={onThemeChange} />
      <Button variant="ghost" onClick={onSettings}>
        Settings
      </Button>
      <Pill>Free · local</Pill>
    </footer>
  );
}
