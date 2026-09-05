import { Logo } from "@harbor/ui/Logo";
import { hostPlatform } from "../platform";
import { BellButton } from "./BellButton";
import { ModeSwitch, type Mode } from "./ModeSwitch";
import { OrbSeat } from "./OrbSeat";
import { SidebarToggle } from "./SidebarToggle";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "./window";
import { WorkspaceMenu } from "./WorkspaceMenu";

export interface TitleBarProps {
  mode: Mode;
  onModeChange: (mode: Mode) => void;
  railOpen: boolean;
  onToggleRail: () => void;
  onOrbClick: () => void;
  onTidy: () => void;
  onBellClick: () => void;
}

export function TitleBar({
  mode,
  onModeChange,
  railOpen,
  onToggleRail,
  onOrbClick,
  onTidy,
  onBellClick,
}: TitleBarProps) {
  const platform = hostPlatform();
  return (
    <header className="harbor-titlebar" data-tauri-drag-region data-platform={platform}>
      <Logo size={18} />
      <ModeSwitch value={mode} onValueChange={onModeChange} />
      <span className="harbor-titlebar-flex" data-tauri-drag-region />
      <WorkspaceMenu onTidy={onTidy} />
      <OrbSeat onClick={onOrbClick} />
      <BellButton onClick={onBellClick} />
      <SidebarToggle open={railOpen} onToggle={onToggleRail} />
      {platform === "windows" ? (
        <div className="harbor-window-controls">
          <button type="button" aria-label="Minimize" onClick={() => void minimizeWindow()}>
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M2 6h8" stroke="currentColor" strokeWidth="1.2" />
            </svg>
          </button>
          <button type="button" aria-label="Maximize" onClick={() => void toggleMaximizeWindow()}>
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <rect x="2.25" y="2.25" width="7.5" height="7.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
            </svg>
          </button>
          <button type="button" aria-label="Close" onClick={() => void closeWindow()}>
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.2" />
            </svg>
          </button>
        </div>
      ) : null}
    </header>
  );
}
