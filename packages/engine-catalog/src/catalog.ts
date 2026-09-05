import type { EngineSpec } from "./types.ts";
import catalogJson from "./catalog.json" with { type: "json" };

export type { ChatMode, DetectedEngine, EngineId, EngineSpec, EngineStatus } from "./types.ts";

export const CATALOG: EngineSpec[] = catalogJson as EngineSpec[];

export function specById(id: string): EngineSpec | undefined {
  return CATALOG.find((spec) => spec.id === id);
}
