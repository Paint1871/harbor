import { useEffect, useId, useRef, useState } from "react";
import type { DetectedEngine } from "@harbor/schema/commands";
import type { AcpConfigOption } from "./useAcpThread";

export interface EnginePickerProps {
  engines: DetectedEngine[];
  engineId: string;
  options: AcpConfigOption[];
  /** Chosen in this session, keyed by option id; falls back to what the engine reports. */
  choices: Record<string, string>;
  busy?: boolean;
  disabled?: boolean;
  onOpen?: () => void;
  /** Omitted where the engine is not the caller's to change, as in an agent's own chat. */
  onEngineChange?: (id: string) => void;
  onOptionChange: (optionId: string, value: string) => void;
}

function labelFor(option: AcpConfigOption, choices: Record<string, string>): string | null {
  const current = choices[option.id] ?? option.currentValue;
  if (!current) return null;
  return option.values.find((value) => value.value === current)?.name ?? current;
}

/**
 * One control for both halves of "who answers this": the engine Harbor launches,
 * and whatever that engine offers once it is running.
 */
export function EnginePicker({ engines, engineId, options, choices, busy = false, disabled = false, onOpen, onEngineChange, onOptionChange }: EnginePickerProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const engineName = engines.find((engine) => engine.id === engineId)?.displayName ?? engineId;
  const summary = options.map((option) => labelFor(option, choices)).find(Boolean) ?? null;

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
        onClick={() => {
          setOpen((value) => {
            if (!value) onOpen?.();
            return !value;
          });
        }}
      >
        <span className="harbor-engine-trigger-name">{engineName}</span>
        {summary ? <span className="harbor-engine-trigger-choice">{summary}</span> : null}
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
          {options.map((option) => {
            const current = choices[option.id] ?? option.currentValue;
            return (
              <div key={option.id}>
                <p className="harbor-engine-menu-label">{option.name}</p>
                {option.values.map((value) => (
                  <button
                    key={value.value}
                    type="button"
                    role="menuitemradio"
                    aria-checked={value.value === current}
                    title={value.description ?? undefined}
                    onClick={() => {
                      setOpen(false);
                      if (value.value !== current) onOptionChange(option.id, value.value);
                    }}
                  >
                    <span>{value.name}</span>
                    {value.value === current ? <span aria-hidden="true">✓</span> : null}
                  </button>
                ))}
              </div>
            );
          })}
          {options.length === 0 ? (
            <p className="harbor-engine-menu-note">
              {busy ? `Asking ${engineName} what it offers…` : `${engineName} offers no model or mode over ACP.`}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
