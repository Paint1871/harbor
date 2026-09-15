import { expect, it } from "vitest";
import { settingMatchesQuery } from "./settingMatchesQuery";

it("treats an empty or blank query as a match", () => {
  expect(settingMatchesQuery("Default shell", "The shell used when a new terminal pane starts.", "")).toBe(true);
  expect(settingMatchesQuery("Default shell", "The shell used when a new terminal pane starts.", "   ")).toBe(true);
});

it("matches label and description case-insensitively", () => {
  expect(settingMatchesQuery("Default shell", "The shell used when a new terminal pane starts.", "shell")).toBe(true);
  expect(settingMatchesQuery("Default shell", "The shell used when a new terminal pane starts.", "SHELL")).toBe(true);
  expect(settingMatchesQuery("Mail", "Handoffs between local agents.", "handoffs")).toBe(true);
  expect(settingMatchesQuery("Mail", "Handoffs between local agents.", "MAIL")).toBe(true);
});

it("rejects settings that do not contain the query", () => {
  expect(settingMatchesQuery("Default shell", "The shell used when a new terminal pane starts.", "xyzzy")).toBe(false);
  expect(settingMatchesQuery("Appearance", "Choose the calm, dark-first canvas or a light paper surface.", "mail")).toBe(false);
});
