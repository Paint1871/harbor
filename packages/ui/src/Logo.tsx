import harborLogoSrc from "./harbor-logo.png";

interface LogoProps {
  size?: number;
  title?: string;
  className?: string;
}

/** The shared Harbor mark used across the desktop chrome and app icon. */
export function Logo({ size = 20, title = "Harbor", className = "" }: LogoProps) {
  return (
    <img
      className={`harbor-logo ${className}`}
      src={harborLogoSrc}
      width={size}
      height={size}
      role="img"
      aria-label={title}
      alt={title}
      draggable={false}
    />
  );
}
