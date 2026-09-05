import type { ReactNode } from "react";
import { Destinations } from "./Destinations";
import { Footer } from "./Footer";

export function AppRail({ children }: { children: ReactNode }) {
  return (
    <aside className="harbor-rail" aria-label="Sidebar">
      <Destinations />
      <div className="harbor-rail-body">{children}</div>
      <Footer />
    </aside>
  );
}
