import { describe, expect, it, vi } from "vitest";
import {
  confirmCloseDirtyTab,
  DIRTY_CLOSE_MESSAGE,
  fileBasename,
  languageIdFor,
} from "./helpers";

describe("fileBasename", () => {
  it("returns the last path segment on POSIX and Windows paths", () => {
    expect(fileBasename("/Users/ada/src/main.rs")).toBe("main.rs");
    expect(fileBasename("C:\\Users\\ada\\src\\main.rs")).toBe("main.rs");
    expect(fileBasename("main.rs")).toBe("main.rs");
    expect(fileBasename("src/")).toBe("src");
  });
});

describe("languageIdFor", () => {
  it("maps known extensions and falls back to plain text", () => {
    expect(languageIdFor("app.py")).toBe("python");
    expect(languageIdFor("src/main.rs")).toBe("rust");
    expect(languageIdFor("styles.css")).toBe("css");
    expect(languageIdFor("index.html")).toBe("html");
    expect(languageIdFor("schema.xml")).toBe("xml");
    expect(languageIdFor("Cargo.toml")).toBe("toml");
    expect(languageIdFor("package.json")).toBe("json");
    expect(languageIdFor("README.md")).toBe("markdown");
    expect(languageIdFor("src/app.ts")).toBe("typescript");
    expect(languageIdFor("src/app.tsx")).toBe("typescript");
    expect(languageIdFor("src/app.jsx")).toBe("javascript");
    expect(languageIdFor("src/app.js")).toBe("javascript");
    expect(languageIdFor("Makefile")).toBe("plaintext");
    expect(languageIdFor("main.go")).toBe("plaintext");
  });
});

describe("confirmCloseDirtyTab", () => {
  it("closes a clean tab without prompting", () => {
    const confirm = vi.fn(() => false);
    expect(confirmCloseDirtyTab(false, confirm)).toBe(true);
    expect(confirm).not.toHaveBeenCalled();
  });

  it("asks before closing a dirty tab", () => {
    const confirm = vi.fn((message: string) => {
      expect(message).toBe(DIRTY_CLOSE_MESSAGE);
      return false;
    });
    expect(confirmCloseDirtyTab(true, confirm)).toBe(false);
    expect(confirm).toHaveBeenCalledOnce();
    expect(confirmCloseDirtyTab(true, () => true)).toBe(true);
  });
});
