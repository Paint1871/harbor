import { Face } from "./Face";

interface FacePickerProps {
  name: string;
  value: number;
  onChange: (index: number) => void;
}

export function FacePicker({ name, value, onChange }: FacePickerProps) {
  return (
    <div className="harbor-face-picker" role="listbox" aria-label="Face">
      {Array.from({ length: 12 }, (_, index) => (
        <button
          key={index}
          type="button"
          aria-selected={index === value}
          onClick={() => onChange(index)}
        >
          <Face name={name} index={index} />
        </button>
      ))}
    </div>
  );
}
