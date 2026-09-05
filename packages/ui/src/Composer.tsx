import type { ComponentPropsWithRef, ReactNode } from "react";
import { Button } from "./Button";

export interface ComposerProps {
  value: string;
  onValueChange: (value: string) => void;
  onSend: (value: string) => void;
  disabled?: boolean;
  controls?: ReactNode;
  className?: string;
  textareaProps?: Omit<ComponentPropsWithRef<"textarea">, "value" | "defaultValue" | "onChange" | "disabled" | "children">;
}

export function Composer({ value, onValueChange, onSend, disabled = false, controls, className = "", textareaProps = {} }: ComposerProps) {
  const { onKeyDown, className: textareaClassName = "", ...inputProps } = textareaProps;
  const canSend = !disabled && value.trim().length > 0;
  return (
    <form
      className={`harbor-composer ${className}`}
      onSubmit={(event) => {
        event.preventDefault();
        if (canSend) onSend(value);
      }}
    >
      <textarea
        aria-label="Message"
        placeholder="Ask anything..."
        rows={2}
        {...inputProps}
        className={`harbor-composer-input ${textareaClassName}`}
        value={value}
        disabled={disabled}
        onChange={(event) => onValueChange(event.target.value)}
        onKeyDown={(event) => {
          onKeyDown?.(event);
          if (event.defaultPrevented || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return;
          if (event.key === "Enter" && !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            if (canSend) event.currentTarget.form?.requestSubmit();
          }
        }}
      />
      <div className="harbor-composer-footer">
        <div className="harbor-composer-controls">{controls}</div>
        <Button type="submit" variant="primary" disabled={!canSend}>Send</Button>
      </div>
    </form>
  );
}
