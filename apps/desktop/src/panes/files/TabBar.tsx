interface TabBarProps {
  files: string[];
  active?: string;
  onSelect: (path: string) => void;
  onClose: (path: string) => void;
}

export function TabBar({ files, active, onSelect, onClose }: TabBarProps) {
  return (
    <div className="harbor-tabs" role="tablist">
      {files.map((file) => (
        <button
          key={file}
          type="button"
          role="tab"
          aria-selected={file === active}
          onClick={() => onSelect(file)}
        >
          {file}
          <span
            onClick={(event) => {
              event.stopPropagation();
              onClose(file);
            }}
          >
            ×
          </span>
        </button>
      ))}
    </div>
  );
}
