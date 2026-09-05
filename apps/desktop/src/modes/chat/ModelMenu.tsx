interface ModelMenuProps {
  options: { id: string; category: string }[];
  value: string | null;
  onChange: (id: string) => void;
}

export function ModelMenu({ options, value, onChange }: ModelMenuProps) {
  const models = options.filter((option) => option.category === "model" || option.category === "mode");
  if (models.length === 0) return null;
  return (
    <label>
      Model
      <select value={value ?? ""} onChange={(event) => onChange(event.target.value)}>
        {models.map((option) => (
          <option key={option.id} value={option.id}>
            {option.id}
          </option>
        ))}
      </select>
    </label>
  );
}
