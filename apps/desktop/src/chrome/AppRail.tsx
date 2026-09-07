import type { ReactNode } from "react";
import { Destinations } from "./Destinations";
import { Footer } from "./Footer";

/**
 * The rail stays mounted when closed so its width can animate. `inert` keeps
 * the hidden content out of the tab order and away from assistive technology.
 */
export function AppRail({
  children,
  className,
  open = true,
}: {
  children: ReactNode;
  className?: string;
  open?: boolean;
}) {
  return (
    <aside
      className={`harbor-rail${className ? ` ${className}` : ""}`}
      aria-label="Sidebar"
      data-open={open}
      inert={!open}
    >
      <Destinations />
      <div className="harbor-rail-body">{children}</div>
      <Footer />
    </aside>
  );
}
