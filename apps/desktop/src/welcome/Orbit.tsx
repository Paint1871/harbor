import type { CSSProperties } from "react";
import { useTheme } from "@harbor/ui/ThemeProvider";

const FACES = ["H", "A", "C", "K", "M", "R"];

/** Decorative orbit. Reduce Motion freezes the ring. */
export function Orbit() {
  const { reducedMotion } = useTheme();
  return (
    <div className="harbor-orbit" data-still={reducedMotion || undefined} aria-hidden="true">
      <div className="harbor-orbit-ring">
        {FACES.map((letter, index) => (
          <span key={letter} style={{ "--i": index } as CSSProperties}>
            {letter}
          </span>
        ))}
      </div>
    </div>
  );
}
