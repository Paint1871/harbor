import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@harbor/ui/Button";

interface WorkspaceMenuProps {
  onTidy: () => void;
}

/** 0.1.0 exposes Tidy Panes only. Flip Terminals and Usage stay hidden. */
export function WorkspaceMenu({ onTidy }: WorkspaceMenuProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const menuId = useId();

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="harbor-workspace-menu" ref={root}>
      <Button
        variant="ghost"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => setOpen((value) => !value)}
      >
        Workspace
      </Button>
      {open ? (
        <div className="harbor-menu" id={menuId} role="menu">
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              onTidy();
              setOpen(false);
            }}
          >
            Tidy Panes
          </button>
        </div>
      ) : null}
    </div>
  );
}
