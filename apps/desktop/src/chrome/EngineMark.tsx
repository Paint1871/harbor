import { useSyncExternalStore, type ReactElement } from "react";
import { invoke } from "@tauri-apps/api/core";

const STROKE = { fill: "none", stroke: "currentColor", strokeWidth: 1.35, strokeLinecap: "round", strokeLinejoin: "round" } as const;

/**
 * A distinguishable mark per engine.
 *
 * These are original Harbor marks, not the vendors' logos: Harbor is Apache-2.0
 * and does not redistribute third-party brand assets. The goal is that two
 * terminals running different CLIs never look alike. Engines without a mark get
 * a monogram tile, so a new catalog entry still reads as its own thing.
 */
const MARKS: Record<string, { tint: string; art: ReactElement }> = {
  "claude-code": {
    tint: "#d08a5a",
    art: <path d="M8 2.4v11.2M2.4 8h11.2M4.05 4.05l7.9 7.9M11.95 4.05l-7.9 7.9" {...STROKE} />,
  },
  codex: {
    tint: "#9aa6b8",
    art: (
      <>
        <circle cx="8" cy="8" r="5.4" {...STROKE} />
        <path d="M6.05 6.05 4.3 8l1.75 1.95M9.95 6.05 11.7 8l-1.75 1.95" {...STROKE} />
      </>
    ),
  },
  opencode: {
    tint: "#6fb8a0",
    art: <path d="M8 2.3 13.3 5.4v5.2L8 13.7 2.7 10.6V5.4Z" {...STROKE} />,
  },
  cursor: {
    tint: "#8ea2c8",
    art: <path d="M4 2.6 12.6 8.1l-3.9.9-1.6 3.7Z" {...STROKE} />,
  },
  gemini: {
    tint: "#7ba3e8",
    art: <path d="M8 2.2c.35 3 2.6 5.25 5.6 5.6-3 .35-5.25 2.6-5.6 5.6-.35-3-2.6-5.25-5.6-5.6 3-.35 5.25-2.6 5.6-5.6Z" {...STROKE} />,
  },
  copilot: {
    tint: "#9b8fc4",
    art: (
      <>
        <path d="M2.6 8.9c0-2.6 2.4-4.5 5.4-4.5s5.4 1.9 5.4 4.5c0 1.9-2.4 3.2-5.4 3.2s-5.4-1.3-5.4-3.2Z" {...STROKE} />
        <path d="M8 4.4c0-1.2.8-2 2-2" {...STROKE} />
      </>
    ),
  },
  "grok-build": {
    tint: "#c98a8a",
    art: <path d="M9.1 2.3 4.2 8.9h3.1l-.6 4.8 5-6.7H8.5Z" {...STROKE} />,
  },
  aider: {
    tint: "#88b39a",
    art: (
      <>
        <circle cx="6.2" cy="8" r="3.5" {...STROKE} />
        <circle cx="9.8" cy="8" r="3.5" {...STROKE} />
      </>
    ),
  },
  amp: {
    tint: "#c2a06a",
    art: <path d="M1.9 8h2.2l1.6-4.4L8 12.6l2.1-6.2 1.3 1.6h2.7" {...STROKE} />,
  },
  factory: {
    tint: "#8f9bb0",
    art: <path d="M2.6 13.2V6.6l3.5 2.4V6.6l3.5 2.4V3.2h3.8v10Z" {...STROKE} />,
  },
  shell: {
    tint: "#8a94a6",
    art: (
      <>
        <rect x="1.9" y="3.1" width="12.2" height="9.8" rx="2" {...STROKE} />
        <path d="M4.8 6.5 6.9 8.3 4.8 10.1M8.9 10.3h2.6" {...STROKE} />
      </>
    ),
  },
};

const MONOGRAM_TINTS = ["#7ba3e8", "#6fb8a0", "#c2a06a", "#9b8fc4", "#c98a8a", "#8f9bb0"];

function monogram(label: string): string {
  const words = label.trim().split(/[\s-]+/).filter(Boolean);
  const [first, second] = words;
  if (!first) return "?";
  if (!second) return first.slice(0, 2).toUpperCase();
  return (first.charAt(0) + second.charAt(0)).toUpperCase();
}


/**
 * Real vendor logos, if the builder installed any.
 *
 * Harbor ships original marks and does not redistribute vendor trademarks, so
 * this store is empty until logos are placed in the engine-icons directory.
 * Loaded once per session; every mark re-renders when it arrives.
 */
let installedIcons: Record<string, string> = {};
const iconListeners = new Set<() => void>();
let iconsRequested = false;

function subscribeIcons(listener: () => void): () => void {
  iconListeners.add(listener);
  if (!iconsRequested) {
    iconsRequested = true;
    void invoke<{ engineId: string; dataUrl: string }[]>("engine_icons")
      .then((rows) => {
        const next: Record<string, string> = {};
        for (const row of rows) next[row.engineId] = row.dataUrl;
        installedIcons = next;
        iconListeners.forEach((notify) => notify());
      })
      .catch(() => undefined);
  }
  return () => {
    iconListeners.delete(listener);
  };
}

function iconSnapshot(): Record<string, string> {
  return installedIcons;
}

/** Test seam: drop any loaded logos so drawn marks are exercised. */
export function resetEngineIconsForTest(): void {
  installedIcons = {};
  iconsRequested = false;
  iconListeners.forEach((notify) => notify());
}

export interface EngineMarkProps {
  engineId?: string | null;
  label: string;
  size?: number;
}

/** A pane can carry a label before its engine id is known; keep the two in step. */
const BY_LABEL: Record<string, string> = {
  "Claude Code": "claude-code",
  Codex: "codex",
  OpenCode: "opencode",
  Cursor: "cursor",
  "Gemini CLI": "gemini",
  "GitHub Copilot": "copilot",
  "Grok Build": "grok-build",
  Aider: "aider",
  Amp: "amp",
  Factory: "factory",
  Shell: "shell",
};

export function resolveEngineId(engineId: string | null | undefined, label: string): string {
  if (engineId && engineId !== "shell" && MARKS[engineId]) return engineId;
  const byLabel = BY_LABEL[label.trim()];
  if (byLabel) return byLabel;
  if (label.startsWith("zsh") || label.startsWith("bash") || label.startsWith("pwsh")) return "shell";
  return engineId ?? "shell";
}

export function EngineMark({ engineId, label, size = 14 }: EngineMarkProps) {
  const resolved = resolveEngineId(engineId, label);
  const icons = useSyncExternalStore(subscribeIcons, iconSnapshot, iconSnapshot);
  const installed = icons[resolved];
  if (installed) {
    return (
      <span className="harbor-engine-mark harbor-engine-logo" style={{ width: size, height: size }}>
        <img src={installed} alt="" width={size} height={size} />
      </span>
    );
  }
  const mark = MARKS[resolved];
  if (mark) {
    return (
      <span className="harbor-engine-mark" style={{ color: mark.tint }}>
        <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
          {mark.art}
        </svg>
      </span>
    );
  }
  const text = monogram(label);
  let hash = 0;
  for (const char of resolved || label) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  const tint = MONOGRAM_TINTS[hash % MONOGRAM_TINTS.length];
  return (
    <span
      className="harbor-engine-mark harbor-engine-monogram"
      style={{ color: tint, width: size + 2, height: size + 2 }}
      aria-hidden="true"
    >
      {text}
    </span>
  );
}
