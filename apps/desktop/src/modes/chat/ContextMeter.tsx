import type { ChatMessage } from "@harbor/schema/commands";

/** Rough, and said to be rough: ~4 characters per token across English prose and code. */
const CHARS_PER_TOKEN = 4;

export function estimateTokens(lines: ChatMessage[]): number {
  return Math.round(lines.reduce((total, line) => total + line.text.length, 0) / CHARS_PER_TOKEN);
}

export function formatTokens(tokens: number): string {
  if (tokens < 1000) return `~${tokens}`;
  const thousands = tokens / 1000;
  return `~${thousands < 10 ? thousands.toFixed(1) : Math.round(thousands)}k`;
}

/**
 * What Harbor is holding for this thread. It deliberately shows no percentage:
 * the engine's context window is its own business and Harbor is not told it.
 */
export function ContextMeter({ lines, attached = 0 }: { lines: ChatMessage[]; attached?: number }) {
  if (!lines.length) return null;
  const tokens = estimateTokens(lines);
  const parts = [
    `${formatTokens(tokens)} tokens`,
    `${lines.length} ${lines.length === 1 ? "message" : "messages"}`,
    attached ? `${attached} attached` : null,
  ].filter((part): part is string => Boolean(part));
  return (
    <span
      className="harbor-context-meter"
      title="Estimated from the text Harbor stores for this thread, at about four characters per token. Your engine counts its own context separately."
    >
      {parts.join(" · ")}
    </span>
  );
}
