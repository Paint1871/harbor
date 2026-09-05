import atlasUrl from "../../../../../assets/faces/atlas.webp";

interface FaceProps {
  name: string;
  index: number;
}

const GRID = 8;

export function Face({ name, index }: FaceProps) {
  const initials = name
    .split(/\s+/)
    .map((part) => part[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  const slot = ((index % 64) + 64) % 64;
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
