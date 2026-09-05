interface FloatingPillProps {
  copy: string;
}

export function FloatingPill({ copy }: FloatingPillProps) {
  return (
    <div className="harbor-pill-overlay" data-kind="floating">
      {copy}
    </div>
  );
}
