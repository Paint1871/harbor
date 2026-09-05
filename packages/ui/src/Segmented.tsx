import { useId } from "react";

export interface SegmentOption<Value extends string> {
  value: Value;
  label: string;
  disabled?: boolean;
}

export interface SegmentedProps<Value extends string> {
  label: string;
  value: Value;
  options: readonly SegmentOption<Value>[];
  onValueChange: (value: Value) => void;
  disabled?: boolean;
  className?: string;
}

/** Native radio inputs provide one tab stop, arrow navigation, and disabled-item skipping. */
export function Segmented<Value extends string>({ label, value, options, onValueChange, disabled = false, className = "" }: SegmentedProps<Value>) {
  const name = useId();
  return (
    <div className={`harbor-segmented ${className}`} role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <label key={option.value} className="harbor-segment">
          <input
            type="radio"
            name={name}
            value={option.value}
            checked={value === option.value}
            disabled={disabled || option.disabled}
            onChange={() => onValueChange(option.value)}
          />
          <span>{option.label}</span>
        </label>
      ))}
    </div>
  );
}
