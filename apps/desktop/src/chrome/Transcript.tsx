export interface TranscriptLine {
  id: string;
  text: string;
  role: "user" | "assistant" | "mail";
}

export function Transcript({ lines }: { lines: TranscriptLine[] }) {
  return (
    <div className="harbor-chat-transcript">
      {lines.map((line) => (
        <div
          key={line.id}
          className={`${line.role === "user" ? "harbor-bubble harbor-bubble-user" : "harbor-assistant-block"} harbor-transcript-line`}
          data-role={line.role === "user" ? "You" : line.role === "mail" ? "Handoff" : "Agent"}
        >
          {line.role === "mail" ? `Mail: ${line.text}` : line.text}
        </div>
      ))}
    </div>
  );
}
