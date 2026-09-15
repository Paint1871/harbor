import { fileBasename } from "./helpers";

interface TabBarProps {
  files: string[];
  active?: string;
  dirty?: readonly string[];
  onSelect: (path: string) => void;
  onClose: (path: string) => void;
}

export function TabBar({ files, active, dirty = [], onSelect, onClose }: TabBarProps) {
  return (
    <div className="harbor-tabs" role="tablist">
      {files.map((file) => {
        const name = fileBasename(file);
        const marked = dirty.includes(file);
        return (
          <button
            key={file}
            type="button"
            role="tab"
            title={file}
            aria-label={marked ? `${name}, unsaved changes` : name}
            aria-selected={file === active}
            data-dirty={marked ? "true" : "false"}
            onClick={() => onSelect(file)}
          >
            {marked ? <span className="harbor-tab-dirty" aria-hidden="true">•</span> : null}
            <span className="harbor-tab-name">{name}</span>
            <span
              className="harbor-tab-close"
              onClick={(event) => {
                event.stopPropagation();
                onClose(file);
              }}
            >
              ×
            </span>
          </button>
        );
      })}
    </div>
  );
}
