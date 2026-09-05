import { Segmented } from "@harbor/ui/Segmented";

export const MODES = [
  { value: "agent", label: "Agent" },
  { value: "code", label: "Code" },
  { value: "chat", label: "Chat" },
] as const;

export type Mode = (typeof MODES)[number]["value"];

interface ModeSwitchProps {
  value: Mode;
  onValueChange: (value: Mode) => void;
}

export function ModeSwitch({ value, onValueChange }: ModeSwitchProps) {
  return <Segmented label="Mode" value={value} options={MODES} onValueChange={onValueChange} />;
}
