import type { ComponentPropsWithRef } from "react";

export interface PillProps extends ComponentPropsWithRef<"span"> {
  tone?: "neutral" | "live" | "attention";
}

export function Pill({ tone = "neutral", className = "", ...props }: PillProps) {
  return <span {...props} className={`harbor-pill ${className}`} data-tone={tone} />;
}
