interface NotchPillProps {
  copy: string;
  tone?: "live" | "error";
}

export function NotchPill({ copy, tone }: NotchPillProps) {
  return (
    <div className="harbor-pill-overlay" data-kind="notch" data-tone={tone}>
      <span className="harbor-pill-pip" aria-hidden="true" />
      {copy}
    </div>
  );
}
