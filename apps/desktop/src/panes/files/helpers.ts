export type EditorLanguage =
  | "javascript"
  | "typescript"
  | "json"
  | "markdown"
  | "python"
  | "rust"
  | "css"
  | "html"
  | "xml"
  | "toml"
  | "plaintext";

export const DIRTY_CLOSE_MESSAGE = "This file has unsaved changes.";

export function fileBasename(path: string): string {
  const parts = path.split(/[\\/]/);
  for (let i = parts.length - 1; i >= 0; i -= 1) {
    const part = parts[i];
    if (part) return part;
  }
  return path;
}

export function fileExtension(path: string): string {
  const name = fileBasename(path);
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return "";
  return name.slice(dot).toLowerCase();
}

export function languageIdFor(path: string): EditorLanguage {
  switch (fileExtension(path)) {
    case ".py":
      return "python";
    case ".rs":
      return "rust";
    case ".css":
      return "css";
    case ".html":
    case ".htm":
      return "html";
    case ".xml":
      return "xml";
    case ".toml":
      return "toml";
    case ".json":
      return "json";
    case ".md":
    case ".markdown":
      return "markdown";
    case ".ts":
    case ".tsx":
    case ".mts":
    case ".cts":
      return "typescript";
    case ".js":
    case ".jsx":
    case ".mjs":
    case ".cjs":
      return "javascript";
    default:
      return "plaintext";
  }
}

export function confirmCloseDirtyTab(
  dirty: boolean,
  confirm: (message: string) => boolean,
): boolean {
  if (!dirty) return true;
  return confirm(DIRTY_CLOSE_MESSAGE);
}
