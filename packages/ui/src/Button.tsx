import type { ComponentPropsWithRef } from "react";

export interface ButtonProps extends ComponentPropsWithRef<"button"> {
  variant?: "primary" | "secondary" | "ghost";
  size?: "default" | "icon";
}

export function Button({ variant = "secondary", size = "default", className = "", type = "button", ...props }: ButtonProps) {
  return <button {...props} type={type} className={`harbor-button ${className}`} data-variant={variant} data-size={size} />;
}
