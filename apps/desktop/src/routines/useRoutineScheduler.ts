import { useEffect } from "react";
import { runDueRoutines } from "./store";

/** Local schedules only exist while Harbor is open, so a quiet poll is enough. */
const TICK_MS = 5 * 60_000;

export function useRoutineScheduler() {
  useEffect(() => {
    let running = false;
    const tick = () => {
      if (running) return;
      running = true;
      void runDueRoutines()
        .catch(() => undefined)
        .finally(() => { running = false; });
    };
    tick();
    const id = setInterval(tick, TICK_MS);
    return () => clearInterval(id);
  }, []);
}
