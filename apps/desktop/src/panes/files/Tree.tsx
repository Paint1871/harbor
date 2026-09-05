interface TreeProps {
  onOpen: (path: string) => void;
}

export function Tree({ onOpen }: TreeProps) {
  return (
    <ul className="harbor-tree">
      <li>
        <button type="button" onClick={() => onOpen("README.md")}>
          README.md
        </button>
      </li>
    </ul>
  );
}
