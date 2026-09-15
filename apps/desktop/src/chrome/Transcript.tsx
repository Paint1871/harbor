import { useEffect, useRef, useState } from "react";

export interface TranscriptLine {
  id: string;
  text: string;
  role: "user" | "assistant" | "mail";
}

/** Visible string for a line; mail keeps the on-screen `Mail: ` prefix. */
export function transcriptCopyText(line: TranscriptLine): string {
  return line.role === "mail" ? `Mail: ${line.text}` : line.text;
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

function TranscriptCopyButton({ line }: { line: TranscriptLine }) {
  const [copied, setCopied] = useState(false);
  const revert = useRef(0);

  useEffect(() => () => window.clearTimeout(revert.current), []);

  async function onCopy() {
    try {
      await copyText(transcriptCopyText(line));
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
      className="harbor-transcript-copy"
      aria-label={copied ? "Copied" : "Copy message"}
      onClick={() => void onCopy()}
    >
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

export function Transcript({ lines }: { lines: TranscriptLine[] }) {
  return (
    <div className="harbor-chat-transcript">
      {lines.map((line) => (
        <div key={line.id} className="harbor-transcript-item">
          <div
            className={`${line.role === "user" ? "harbor-bubble harbor-bubble-user" : "harbor-assistant-block"} harbor-transcript-line`}
            data-role={line.role === "user" ? "You" : line.role === "mail" ? "Handoff" : "Agent"}
          >
            {transcriptCopyText(line)}
          </div>
          <TranscriptCopyButton line={line} />
        </div>
      ))}
    </div>
  );
}
