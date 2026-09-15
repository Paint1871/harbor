import { expect, it } from "vitest";
import { memoryMatchesQuery } from "./memoryMatchesQuery";

it("treats an empty or blank query as a match", () => {
  expect(memoryMatchesQuery("Prefers tests before merge", "")).toBe(true);
  expect(memoryMatchesQuery("Prefers tests before merge", "   ")).toBe(true);
});

it("matches a fact body case-insensitively", () => {
  expect(memoryMatchesQuery("Prefers tests before merge", "tests")).toBe(true);
  expect(memoryMatchesQuery("Prefers tests before merge", "TESTS")).toBe(true);
  expect(memoryMatchesQuery("Uses conventional commits", "commit")).toBe(true);
});

it("rejects facts that do not contain the query", () => {
  expect(memoryMatchesQuery("Prefers tests before merge", "xyzzy")).toBe(false);
  expect(memoryMatchesQuery("Uses conventional commits", "tests")).toBe(false);
});
