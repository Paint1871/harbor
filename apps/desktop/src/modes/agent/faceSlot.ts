export const FACE_SLOT_COUNT = 64;

const FNV_OFFSET = 0x811c9dc5;
const FNV_PRIME = 0x01000193;

/** Stable atlas slot 0..63 from an agent id (FNV-1a 32-bit over UTF-8 bytes). */
export function faceSlot(agentId: string): number {
  let hash = FNV_OFFSET >>> 0;
  for (const byte of new TextEncoder().encode(agentId)) {
    hash ^= byte;
    hash = Math.imul(hash, FNV_PRIME) >>> 0;
  }
  return hash % FACE_SLOT_COUNT;
}

export function wrapFaceSlot(index: number): number {
  return ((index % FACE_SLOT_COUNT) + FACE_SLOT_COUNT) % FACE_SLOT_COUNT;
}
