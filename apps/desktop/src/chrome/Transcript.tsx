export interface TranscriptLine {
  id: string;
  text: string;
  role: "user" | "assistant";
}

export function Transcript({ lines }: { lines: TranscriptLine[] }) {
  return (
    <div className="harbor-chat-transcript">
      {lines.map((line) => (
        <div
          key={line.id}
          className={
            line.role === "user" ? "harbor-bubble harbor-bubble-user" : "harbor-assistant-block"
          }
        >
          {line.text}
        </div>
      ))}
    </div>
  );
}
