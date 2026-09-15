import { useEffect, useRef, useState } from "react";
import { call } from "../../ipc";
import type { FileDiff } from "@harbor/schema/commands";

interface ChangesPanelProps {
  workspaceId: string | null;
  refreshToken: number;
}

export function changesSummaryLabel(count: number): string {
  return count > 0 ? `Changes · ${count}` : "Changes";
}

async function copyText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  try {
    if (!document.execCommand("copy")) throw new Error("Clipboard access is unavailable in this window.");
  } finally {
    textarea.remove();
  }
}

function ChangeCopyButton({ patch }: { patch: string }) {
  const [copied, setCopied] = useState(false);
  const revert = useRef(0);

  useEffect(() => () => window.clearTimeout(revert.current), []);

  async function onCopy() {
    try {
      await copyText(patch);
    } catch {
      setCopied(false);
      return;
    }
    setCopied(true);
    window.clearTimeout(revert.current);
    revert.current = window.setTimeout(() => setCopied(false), 2000);
  }

  return (
    <button
      type="button"
      className="harbor-changes-copy"
      aria-label={copied ? "Copied" : "Copy"}
      onClick={() => void onCopy()}
    >
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

function matchesChangeQuery(path: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return path.toLowerCase().includes(needle);
}

export function ChangesPanel({ workspaceId, refreshToken }: ChangesPanelProps) {
  const [open, setOpen] = useState(false);
  const [diffs, setDiffs] = useState<FileDiff[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (!workspaceId) {
      setDiffs([]);
      return;
    }
    void call("git_diff", { workspaceId })
      .then(setDiffs)
      .catch(() => setDiffs([]));
  }, [workspaceId, refreshToken]);

  const visible = diffs.filter((diff) => matchesChangeQuery(diff.path, query));

  return (
    <details className="harbor-changes" open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>{changesSummaryLabel(diffs.length)}</summary>
      {diffs.length === 0 ? <p className="harbor-muted">No changes yet.</p> : (
        <>
          <label className="harbor-changes-find">
            Find a change
            <input
              aria-label="Find a change"
              placeholder="Find a change"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          {visible.length === 0 ? <p className="harbor-muted">No changes match “{query}”.</p> : visible.map((diff) => (
            <article key={diff.path}>
              <div className="harbor-changes-heading">
                <h3>{diff.path}</h3>
                <ChangeCopyButton patch={diff.patch} />
              </div>
              <pre>{diff.patch}</pre>
            </article>
          ))}
        </>
      )}
    </details>
  );
}
