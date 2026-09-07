import { useEffect, useRef, useState } from "react";

interface OrbSeatProps {
  onOpenVoiceSettings: () => void;
}

/**
 * Mute chrome for 0.1.0 (K16): a 28px seat in the 44px title bar, left of the
 * bell (K30). Click states that voice is off and points at Settings → Voice.
 * Hold is a no-op until the voice copilot ships.
 */
export function OrbSeat({ onOpenVoiceSettings }: OrbSeatProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="harbor-orb-seat-root" ref={root}>
      <button
        type="button"
        className="harbor-orb-seat"
        aria-label="Voice"
        aria-expanded={open}
        data-phase="mute"
        onClick={() => setOpen((value) => !value)}
      >
        <span className="harbor-orb" aria-hidden="true" />
      </button>
      {open ? (
        <div className="harbor-orb-notice" role="status">
          <p>Voice is off until a later release</p>
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              onOpenVoiceSettings();
            }}
          >
            Settings → Voice
          </button>
        </div>
      ) : null}
    </div>
  );
}
