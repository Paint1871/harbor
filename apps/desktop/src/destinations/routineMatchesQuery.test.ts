import { expect, it } from "vitest";
import { routineMatchesQuery } from "./routineMatchesQuery";

const routine = { name: "Morning scan", brief: "Summarize the open decisions." };

it("treats an empty or blank query as a match", () => {
  expect(routineMatchesQuery(routine, "")).toBe(true);
  expect(routineMatchesQuery(routine, "   ")).toBe(true);
});

it("matches name and brief case-insensitively", () => {
  expect(routineMatchesQuery(routine, "morning")).toBe(true);
  expect(routineMatchesQuery(routine, "SCAN")).toBe(true);
  expect(routineMatchesQuery(routine, "decisions")).toBe(true);
  expect(routineMatchesQuery(routine, "OPEN DECISIONS")).toBe(true);
});

it("rejects routines that do not contain the query", () => {
  expect(routineMatchesQuery(routine, "xyzzy")).toBe(false);
  expect(routineMatchesQuery({ name: "Weekly notes", brief: "Collect release notes" }, "morning")).toBe(false);
});
