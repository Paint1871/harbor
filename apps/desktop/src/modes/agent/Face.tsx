import atlasUrl from "../../../../../assets/faces/atlas.webp";
import { faceSlot, wrapFaceSlot } from "./faceSlot";

interface FaceProps {
  name: string;
  index?: number;
  id?: string;
}

const GRID = 8;

export function Face({ name, index, id }: FaceProps) {
  const initials = name
    .split(/\s+/)
    .map((part) => part[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  const slot = index != null ? wrapFaceSlot(index) : id ? faceSlot(id) : 0;
  const col = slot % GRID;
  const row = Math.floor(slot / GRID);
  return (
    <span
      className="harbor-face"
      aria-hidden="true"
      style={{
        backgroundImage: `url(${atlasUrl})`,
        backgroundSize: `${GRID * 100}% ${GRID * 100}%`,
        backgroundPosition: `${(col / (GRID - 1)) * 100}% ${(row / (GRID - 1)) * 100}%`,
      }}
    >
      {initials}
    </span>
  );
}
