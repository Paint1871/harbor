import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FileDiff } from "@harbor/schema/commands";

interface ChangesPanelProps {
  workspaceId: string | null;
  refreshToken: number;
}

export function ChangesPanel({ workspaceId, refreshToken }: ChangesPanelProps) {
  const [open, setOpen] = useState(false);
  const [diffs, setDiffs] = useState<FileDiff[]>([]);

  useEffect(() => {
    if (!workspaceId || !open) return;
    void invoke<FileDiff[]>("git_diff", { workspaceId })
      .then(setDiffs)
      .catch(() => setDiffs([]));
  }, [workspaceId, open, refreshToken]);

  return (
    <details className="harbor-changes" open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>Changes</summary>
      {diffs.length === 0 ? <p className="harbor-muted">No changes yet.</p> : null}
      {diffs.map((diff) => (
        <article key={diff.path}>
          <h3>{diff.path}</h3>
          <pre>{diff.patch}</pre>
        </article>
      ))}
    </details>
  );
}
