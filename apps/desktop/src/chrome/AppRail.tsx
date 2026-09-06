import type { ReactNode } from "react";
import { Destinations } from "./Destinations";
import { Footer } from "./Footer";

export function AppRail({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <aside className={`harbor-rail${className ? ` ${className}` : ""}`} aria-label="Sidebar">
      <Destinations />
      <div className="harbor-rail-body">{children}</div>
      <Footer />
    </aside>
  );
}
