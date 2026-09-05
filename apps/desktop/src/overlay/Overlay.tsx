import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { FloatingPill } from "./FloatingPill";
import { NotchPill } from "./NotchPill";

interface DictationPayload {
  state: "listening" | "transcribing" | "error" | "idle";
  copy: string;
}

export function Overlay() {
  const [copy, setCopy] = useState("Listening · release fn to send");
  useEffect(() => {
    const unlisten = listen<DictationPayload>("dictation_state", (event) => {
      if (event.payload.copy) setCopy(event.payload.copy);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
  const wide = matchMedia("(min-width: 300px)").matches;
  return wide ? <NotchPill copy={copy} /> : <FloatingPill copy={copy} />;
}
