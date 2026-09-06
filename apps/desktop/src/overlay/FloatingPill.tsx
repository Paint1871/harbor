interface FloatingPillProps {
  copy: string;
  tone?: "live" | "error";
}

export function FloatingPill({ copy, tone }: FloatingPillProps) {
  return (
    <div className="harbor-pill-overlay" data-kind="floating" data-tone={tone}>
      <span className="harbor-pill-pip" aria-hidden="true" />
      {copy}
    </div>
  );
}
