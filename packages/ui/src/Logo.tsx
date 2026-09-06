interface LogoProps {
  size?: number;
  title?: string;
  className?: string;
}

/** A compact two-tone bolt mark that stays legible in the native title bar. */
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
      <path d="M16 2.2 27.2 8.6v14.8L16 29.8 4.8 23.4V8.6L16 2.2Z" fill="#20252f" />
      <path d="M13.7 4.4 7.7 15.1h5.2l-2.1 11.2 7.2-11.9h-5.1l2.1-10Z" fill="#F6C445" />
      <path d="m18.1 4.2-3 10.3h4.8l-2.1 11.1 7-11.8h-4.7l2.1-9.4Z" fill="#5D8DFF" />
    </svg>
  );
}
