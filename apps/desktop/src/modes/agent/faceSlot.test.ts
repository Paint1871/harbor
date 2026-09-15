import { describe, expect, it } from "vitest";
import { FACE_SLOT_COUNT, faceSlot, wrapFaceSlot } from "./faceSlot";

describe("faceSlot", () => {
  it("maps an agent id to a stable atlas slot in 0..63", () => {
    const id = "0193a1c2-7b8d-7e0f-b123-456789abcdef";
    const slot = faceSlot(id);
    expect(slot).toBe(faceSlot(id));
    expect(slot).toBeGreaterThanOrEqual(0);
    expect(slot).toBeLessThan(FACE_SLOT_COUNT);
    const slots = new Set(Array.from({ length: 64 }, (_, i) => faceSlot(`agent-${i}`)));
    expect(slots.size).toBeGreaterThan(1);
    for (let i = 0; i < 200; i += 1) {
      const next = faceSlot(`agent-${i}`);
      expect(next).toBeGreaterThanOrEqual(0);
      expect(next).toBeLessThan(FACE_SLOT_COUNT);
    }
  });

  it("wraps stored indexes onto the atlas", () => {
    expect(wrapFaceSlot(0)).toBe(0);
    expect(wrapFaceSlot(64)).toBe(0);
    expect(wrapFaceSlot(-1)).toBe(63);
  });
});
