import type { ComponentPropsWithRef, ReactNode } from "react";

export interface RailRowProps extends Omit<ComponentPropsWithRef<"button">, "children"> {
  label: string;
  description?: ReactNode;
  leading?: ReactNode;
  trailing?: ReactNode;
  selected?: boolean;
}

/** Slots are presentational; keep additional interactive controls outside this button. */
export function RailRow({ label, description, leading, trailing, selected = false, className = "", type = "button", ...props }: RailRowProps) {
  return (
    <button {...props} type={type} className={`harbor-rail-row ${className}`} aria-current={selected ? true : undefined} title={props.title ?? label}>
      {leading != null ? <span className="harbor-rail-leading">{leading}</span> : null}
      <span className="harbor-rail-copy">
        <span className="harbor-rail-label">{label}</span>
        {description != null ? <span className="harbor-rail-description">{description}</span> : null}
      </span>
      {trailing != null ? <span className="harbor-rail-trailing">{trailing}</span> : null}
    </button>
  );
}
