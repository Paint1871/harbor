interface FaceProps {
  name: string;
  index: number;
}

const HUES = [200, 220, 260, 280, 320, 20, 40, 160, 180, 140, 100, 80];

export function Face({ name, index }: FaceProps) {
  const initials = name
    .split(/\s+/)
    .map((part) => part[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
  const hue = HUES[index % HUES.length] ?? 200;
  return (
    <span className="harbor-face" style={{ background: `hsl(${hue} 30% 28%)` }} aria-hidden="true">
      {initials}
    </span>
  );
}
