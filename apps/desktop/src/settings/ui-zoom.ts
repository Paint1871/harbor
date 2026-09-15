export const UI_ZOOM_PERCENTS = [90, 100, 110, 125] as const;
export type UiZoomPercent = (typeof UI_ZOOM_PERCENTS)[number];

export const UI_ZOOM_PROPERTY = "--harbor-ui-zoom";

function asFiniteNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string") {
    const parsed = Number(value.trim());
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

export function parseUiZoomPercent(value: unknown): UiZoomPercent {
  const parsed = asFiniteNumber(value);
  return parsed !== null && (UI_ZOOM_PERCENTS as readonly number[]).includes(parsed)
    ? parsed as UiZoomPercent
    : 100;
}

export function uiZoomFactor(value: unknown): number {
  return parseUiZoomPercent(value) / 100;
}

export function applyUiZoom(
  value: unknown,
  root: HTMLElement = document.documentElement,
): number {
  const factor = uiZoomFactor(value);
  root.style.setProperty(UI_ZOOM_PROPERTY, String(factor));
  return factor;
}
