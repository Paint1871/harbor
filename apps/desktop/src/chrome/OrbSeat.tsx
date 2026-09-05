interface OrbSeatProps {
  onClick: () => void;
}

/** Mute chrome in 0.1.0. Hold is a no-op until Voice ships. */
export function OrbSeat({ onClick }: OrbSeatProps) {
  return (
    <button type="button" className="harbor-orb-seat" aria-label="Voice" onClick={onClick}>
      <span className="harbor-orb" aria-hidden="true" />
    </button>
  );
}
