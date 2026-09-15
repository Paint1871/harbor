import { useEffect, useRef, useState } from "react";
import { fileBasename } from "./helpers";

interface TabBarProps {
  files: string[];
  active?: string;
  dirty?: readonly string[];
  onSelect: (path: string) => void;
  onClose: (path: string) => void;
}

/** Full path string the tab stores. */
export function tabCopyPath(file: string): string {
  return file;
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

function TabCopyPathButton({ file }: { file: string }) {
  const [copied, setCopied] = useState(false);
  const revert = useRef(0);

  useEffect(() => () => window.clearTimeout(revert.current), []);

  async function onCopy() {
    try {
      await copyText(tabCopyPath(file));
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
      className="harbor-tab-copy"
      aria-label={copied ? "Copied" : `Copy path ${file}`}
      onClick={(event) => {
        event.stopPropagation();
        void onCopy();
      }}
    >
      {copied ? "Copied" : "Copy path"}
    </button>
  );
}

export function TabBar({ files, active, dirty = [], onSelect, onClose }: TabBarProps) {
  return (
    <div className="harbor-tabs" role="tablist">
      {files.map((file) => {
        const name = fileBasename(file);
        const marked = dirty.includes(file);
        return (
          <div
            key={file}
            role="tab"
            title={file}
            tabIndex={0}
            aria-label={marked ? `${name}, unsaved changes` : name}
            aria-selected={file === active}
            data-dirty={marked ? "true" : "false"}
            onClick={() => onSelect(file)}
            onKeyDown={(event) => {
              if (event.currentTarget !== event.target) return;
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelect(file);
              }
            }}
          >
            {marked ? <span className="harbor-tab-dirty" aria-hidden="true">•</span> : null}
            <span className="harbor-tab-name">{name}</span>
            <TabCopyPathButton file={file} />
            <span
              className="harbor-tab-close"
              onClick={(event) => {
                event.stopPropagation();
                onClose(file);
              }}
            >
              ×
            </span>
          </div>
        );
      })}
    </div>
  );
}
