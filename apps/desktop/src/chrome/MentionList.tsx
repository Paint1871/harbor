export interface MentionItem {
  id: string;
  label: string;
}

export function mentionQuery(value: string): string | null {
  const match = /(?:^|\s)@([^\s]*)$/.exec(value);
  return match ? (match[1] ?? "") : null;
}

export function MentionList({
  items,
  label,
  onPick,
}: {
  items: MentionItem[];
  label: string;
  onPick: (id: string) => void;
}) {
  if (!items.length) return null;
  return (
    <ul className="harbor-tree" role="listbox" aria-label={label}>
      {items.map((item) => (
        <li key={item.id}>
          <button type="button" onClick={() => onPick(item.id)}>
            {item.label}
          </button>
        </li>
      ))}
    </ul>
  );
}
