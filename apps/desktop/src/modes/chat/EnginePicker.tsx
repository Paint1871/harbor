import { useEffect, useId, useRef, useState } from "react";
import type { DetectedEngine } from "@harbor/schema/commands";
import type { AcpConfigOption } from "./useAcpThread";

const CATEGORY_LABELS: Record<string, string> = { model: "Model", mode: "Mode", effort: "Effort" };
const CATEGORY_ORDER = ["model", "mode", "effort"];

function categoriesOf(options: AcpConfigOption[]): string[] {
  const seen = options.map((option) => option.category).filter((category) => category in CATEGORY_LABELS);
  return CATEGORY_ORDER.filter((category) => seen.includes(category));
}

export interface EnginePickerProps {
  engines: DetectedEngine[];
  engineId: string;
  options: AcpConfigOption[];
  choice: string | null;
  disabled?: boolean;
  /** Omitted where the engine is not the caller's to change, as in an agent's own chat. */
  onEngineChange?: (id: string) => void;
  onOptionChange: (id: string) => void;
}

/**
 * One control for both halves of "who answers this": the engine Harbor launches,
 * and whatever that engine lets us set once it is running.
 */
export function EnginePicker({ engines, engineId, options, choice, disabled = false, onEngineChange, onOptionChange }: EnginePickerProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const categories = categoriesOf(options);
  const engineName = engines.find((engine) => engine.id === engineId)?.displayName ?? engineId;

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="harbor-engine-picker" ref={root}>
      <button
        type="button"
        className="harbor-engine-trigger"
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="harbor-engine-trigger-name">{engineName}</span>
        {choice ? <span className="harbor-engine-trigger-choice">{choice}</span> : null}
        <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2.5 4 5 6.5 7.5 4" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
      {open ? (
        <div className="harbor-menu harbor-engine-menu" id={menuId} role="menu">
          {onEngineChange ? (
            <>
              <p className="harbor-engine-menu-label">Engine</p>
              {engines.map((engine) => (
                <button
                  key={engine.id}
                  type="button"
                  role="menuitemradio"
                  aria-checked={engine.id === engineId}
                  onClick={() => {
                    setOpen(false);
                    if (engine.id !== engineId) onEngineChange(engine.id);
                  }}
                >
                  <span>{engine.displayName}</span>
                  {engine.id === engineId ? <span aria-hidden="true">✓</span> : null}
                </button>
              ))}
              <p className="harbor-engine-menu-note">
                Another engine starts a fresh session. The transcript stays here; the engine will not have read it.
              </p>
            </>
          ) : null}
          {categories.map((category) => (
            <div key={category}>
              <p className="harbor-engine-menu-label">{CATEGORY_LABELS[category]}</p>
              {options.filter((option) => option.category === category).map((option) => (
                <button
                  key={option.id}
                  type="button"
                  role="menuitemradio"
                  aria-checked={option.id === choice}
                  onClick={() => {
                    setOpen(false);
                    onOptionChange(option.id);
                  }}
                >
                  <span>{option.id}</span>
                  {option.id === choice ? <span aria-hidden="true">✓</span> : null}
                </button>
              ))}
            </div>
          ))}
          {categories.length === 0 ? (
            <p className="harbor-engine-menu-note">{engineName} does not offer a model or mode over ACP.</p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
