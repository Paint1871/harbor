interface LogoProps {
  size?: number;
  title?: string;
  className?: string;
}

/** Geometric harbor mouth: open ring and inner beacon. */
export function Logo({ size = 20, title = "Harbor", className = "" }: LogoProps) {
  return (
    <svg
      className={`harbor-logo ${className}`}
      width={size}
      height={size}
      viewBox="0 0 32 32"
      role="img"
      aria-label={title}
    >
      <title>{title}</title>
      <circle cx="16" cy="16" r="3.2" fill="currentColor" />
      <path
        d="M10.91 25.19A10.5 10.5 0 1 1 21.09 25.19"
        fill="none"
        stroke="currentColor"
        strokeWidth="3.2"
        strokeLinecap="round"
      />
    </svg>
  );
}
