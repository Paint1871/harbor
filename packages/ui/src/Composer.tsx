import { useRef } from "react";
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

/** How long after a send a repeat of that exact text is treated as a write-back. */
const ECHO_WINDOW_MS = 1000;

export function Composer({ value, onValueChange, onSend, disabled = false, controls, className = "", textareaProps = {} }: ComposerProps) {
  const { onKeyDown, className: textareaClassName = "", ...inputProps } = textareaProps;
  const canSend = !disabled && value.trim().length > 0;
  // A sent message can come back on its own: the platform's text engine holds a
  // pending correction across the submit and writes it into the box we cleared,
  // which reads as a message that never went out. Only the exact text just
  // sent, only within a moment of sending, is refused — nobody retypes a whole
  // message that fast, and everything else is the builder still writing.
  const echo = useRef<{ text: string; at: number } | null>(null);

  function submit() {
    if (!canSend) return;
    echo.current = { text: value, at: Date.now() };
    onSend(value);
  }

  function change(next: string) {
    const sent = echo.current;
    if (sent && next === sent.text && Date.now() - sent.at < ECHO_WINDOW_MS) {
      echo.current = null;
      onValueChange("");
      return;
    }
    echo.current = null;
    onValueChange(next);
  }

  return (
    <form
      className={`harbor-composer ${className}`}
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <textarea
        aria-label="Message"
        placeholder="Ask anything..."
        enterKeyHint="send"
        rows={2}
        // macOS keeps an autocorrect suggestion alive across a submit and
        // writes it back into the box we just cleared, which reads as a message
        // that never sent. Spellcheck stays on; it only underlines.
        autoCorrect="off"
        autoCapitalize="off"
        {...inputProps}
        className={`harbor-composer-input ${textareaClassName}`}
        value={value}
        disabled={disabled}
        onChange={(event) => change(event.target.value)}
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
        <Button type="submit" variant="primary" size="icon" aria-label="Send" disabled={!canSend}>
          <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 12.5V3.5M8 3.5 4.5 7M8 3.5 11.5 7" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </Button>
      </div>
    </form>
  );
}
