import { Logo } from "@harbor/ui/Logo";
import { hostPlatform } from "../platform";
import { BellButton } from "./BellButton";
import { ModeSwitch, type Mode } from "./ModeSwitch";
import { SidebarToggle } from "./SidebarToggle";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "./window";

export interface TitleBarProps {
  mode: Mode;
  onModeChange: (mode: Mode) => void;
  railOpen: boolean;
  onToggleRail: () => void;
  onTidy: () => void;
  onBellClick: () => void;
}

export function TitleBar({
  mode,
  onModeChange,
  railOpen,
  onToggleRail,
  onTidy,
  onBellClick,
}: TitleBarProps) {
  const platform = hostPlatform();

  return (
    <header className="harbor-titlebar" data-tauri-drag-region data-platform={platform}>
      <div className="harbor-titlebar-start">
        <span className="harbor-wordmark">
          <Logo size={16} />
          <span className="harbor-wordmark-label">Harbor</span>
        </span>
        <SidebarToggle open={railOpen} onToggle={onToggleRail} />
      </div>
      <div className="harbor-titlebar-center">
        <ModeSwitch value={mode} onValueChange={onModeChange} onValueClick={(next) => {
          if (next === mode) onModeChange(next);
        }} />
      </div>
      <div className="harbor-titlebar-flex" data-tauri-drag-region />
      <div className="harbor-titlebar-end">
        {mode === "code" ? (
          <button type="button" className="harbor-titlebar-tidy" onClick={onTidy} aria-label="Tidy code panes">
            <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 3h4v4H3zM9 3h4v4H9zM3 9h4v4H3zM9 9h4v4H9z" fill="none" stroke="currentColor" strokeWidth="1.15" />
            </svg>
            <span>Tidy</span>
          </button>
        ) : null}
        <BellButton onClick={onBellClick} />
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
      </div>
    </header>
  );
}
