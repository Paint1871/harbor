interface ModelMenuProps {
  options: { id: string; category: string }[];
  value: string | null;
  onChange: (id: string) => void;
}

export function ModelMenu({ options, value, onChange }: ModelMenuProps) {
  const models = options.filter((option) => option.category === "model" || option.category === "mode" || option.category === "effort");
  if (models.length === 0) return null;
  return (
    <label className="harbor-model-menu">
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
