import type { DetectedEngine } from "@harbor/schema/commands";
import { MenuPill, type MenuItem } from "./MenuPill";
import type { AcpConfigOption } from "./useAcpThread";

export interface EngineControlsProps {
  engines: DetectedEngine[];
  /** Chat-capable once their adapter is fetched. */
  installable?: DetectedEngine[];
  installing?: string | null;
  onInstall?: (id: string) => void;
  engineId: string;
  engineName?: string;
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

/**
 * A provider-qualified id like `alibaba-token-plan/qwen3.8-max` carries its own
 * grouping. Using it keeps a thirty-model list readable without a hand-kept map.
 */
function itemsFor(option: AcpConfigOption): MenuItem[] {
  const qualified = option.values.length > 1 && option.values.every((value) => value.value.includes("/"));
  return option.values.map((value) => ({
    id: value.value,
    name: qualified ? value.name.split("/").slice(1).join("/") || value.name : value.name,
    description: value.description,
    group: qualified ? value.name.split("/")[0] ?? null : null,
  }));
}

function valueLabel(option: AcpConfigOption, choices: Record<string, string>): string {
  const current = choices[option.id] ?? option.currentValue;
  const match = option.values.find((value) => value.value === current);
  if (!match) return current ?? option.name;
  return match.name.includes("/") ? match.name.split("/").slice(1).join("/") : match.name;
}

/**
 * One pill per decision, the way the composer's other affordances read: what
 * answers, which model, how hard it thinks. A single menu holding all of them
 * hides the two that matter behind thirty that do not.
 */
export function EngineControls({ engines, installable = [], installing = null, onInstall, engineId, engineName, options, choices, busy = false, disabled = false, onOpen, onEngineChange, onOptionChange }: EngineControlsProps) {
  const name = engineName ?? engines.find((engine) => engine.id === engineId)?.displayName ?? engineId;

  return (
    <>
      {onEngineChange ? (
        <MenuPill
          label="Engine"
          value={name}
          items={engines.map((engine) => ({ id: engine.id, name: engine.displayName }))}
          current={engineId}
          disabled={disabled}
          onOpen={onOpen}
          onSelect={onEngineChange}
          emptyNote="No installed engine speaks ACP."
          footer={
            <div className="harbor-pill-menu-foot">
              {installable.map((engine) => (
                <div key={engine.id} className="harbor-engine-install">
                  <span>{engine.displayName}</span>
                  <button
                    type="button"
                    disabled={installing !== null}
                    title={engine.adapterPackage ?? undefined}
                    onClick={() => onInstall?.(engine.id)}
                  >
                    {installing === engine.id ? "Adding…" : "Add"}
                  </button>
                </div>
              ))}
              <p className="harbor-pill-menu-note">
                A different engine starts a fresh session and has not read this transcript.
                {installable.length ? " Adding one fetches its adapter into Harbor, not onto your system." : ""}
              </p>
            </div>
          }
        />
      ) : (
        <span className="harbor-pill-static">{name}</span>
      )}
      {options.map((option) => (option.settable === false ? (
        <span key={option.id} className="harbor-pill-static" title={`${option.name} is set inside ${name}, not from here`}>
          <span className="harbor-pill-trigger-label">{option.name}</span> {valueLabel(option, choices)}
        </span>
      ) : (
        <MenuPill
          key={option.id}
          label={option.name}
          value={valueLabel(option, choices)}
          items={itemsFor(option)}
          current={choices[option.id] ?? option.currentValue}
          disabled={disabled}
          busy={busy}
          tone="quiet"
          showLabel={option.category !== "model"}
          onSelect={(value) => onOptionChange(option.id, value)}
        />
      )))}
    </>
  );
}
