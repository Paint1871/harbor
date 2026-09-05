import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EditorProps {
  path?: string;
}

export function Editor({ path }: EditorProps) {
  const [text, setText] = useState("");

  useEffect(() => {
    if (!path) {
      setText("");
      return;
    }
    void invoke<string>("fs_read", { workspaceId: "current", path })
      .then(setText)
      .catch(() => setText(`// ${path}\n`));
  }, [path]);

  if (!path) return <p className="harbor-muted">Open a file.</p>;
  return (
    <textarea
      className="harbor-editor"
      aria-label={path}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => void invoke("fs_write", { workspaceId: "current", path, contents: text }).catch(() => undefined)}
    />
  );
}
