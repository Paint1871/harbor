interface NotchPillProps {
  copy: string;
}

export function NotchPill({ copy }: NotchPillProps) {
  return (
    <div className="harbor-pill-overlay" data-kind="notch">
      {copy}
    </div>
  );
}
