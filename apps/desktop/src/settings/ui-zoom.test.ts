// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { applyUiZoom, parseUiZoomPercent, UI_ZOOM_PROPERTY, uiZoomFactor } from "./ui-zoom";

const css = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "../app.css"), "utf8");

afterEach(() => {
  document.documentElement.style.removeProperty(UI_ZOOM_PROPERTY);
});

describe("parseUiZoomPercent", () => {
  it("accepts the four General percents as numbers or numeric strings", () => {
    expect(parseUiZoomPercent(90)).toBe(90);
    expect(parseUiZoomPercent(100)).toBe(100);
    expect(parseUiZoomPercent(110)).toBe(110);
    expect(parseUiZoomPercent(125)).toBe(125);
    expect(parseUiZoomPercent("90")).toBe(90);
    expect(parseUiZoomPercent("100")).toBe(100);
    expect(parseUiZoomPercent("110")).toBe(110);
    expect(parseUiZoomPercent(" 125 ")).toBe(125);
  });

  it("falls back to 100 for anything else", () => {
    expect(parseUiZoomPercent(undefined)).toBe(100);
    expect(parseUiZoomPercent(null)).toBe(100);
    expect(parseUiZoomPercent("")).toBe(100);
    expect(parseUiZoomPercent("110%")).toBe(100);
    expect(parseUiZoomPercent(1.1)).toBe(100);
    expect(parseUiZoomPercent(105)).toBe(100);
    expect(parseUiZoomPercent(200)).toBe(100);
    expect(parseUiZoomPercent(true)).toBe(100);
    expect(parseUiZoomPercent({ value: 110 })).toBe(100);
  });
});

describe("applyUiZoom", () => {
  it("writes a unitless factor onto the document root", () => {
    expect(applyUiZoom("110")).toBe(1.1);
    expect(document.documentElement.style.getPropertyValue(UI_ZOOM_PROPERTY)).toBe("1.1");
    expect(uiZoomFactor(90)).toBe(0.9);
    expect(uiZoomFactor("125")).toBe(1.25);
    applyUiZoom(0);
    expect(document.documentElement.style.getPropertyValue(UI_ZOOM_PROPERTY)).toBe("1");
  });
});

describe("chrome CSS", () => {
  it("consumes --harbor-ui-zoom with CSS zoom on the app and welcome roots", () => {
    expect(css).toContain(`${UI_ZOOM_PROPERTY}: 1`);
    expect(css.match(/zoom:\s*var\(--harbor-ui-zoom\)/g)?.length).toBeGreaterThanOrEqual(1);
    expect(css).toMatch(/\.harbor-app,\s*\n\s*\.harbor-welcome\s*\{/);
  });
});
