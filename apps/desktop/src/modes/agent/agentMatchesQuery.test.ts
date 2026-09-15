import { expect, it } from "vitest";
import { agentMatchesQuery } from "./agentMatchesQuery";

const agent = { name: "Release manager", brief: "Ship the launch", lastLine: "Waiting on review" };

it("treats an empty or blank query as a match", () => {
  expect(agentMatchesQuery(agent, "")).toBe(true);
  expect(agentMatchesQuery(agent, "   ")).toBe(true);
});

it("matches name, brief, and last line case-insensitively", () => {
  expect(agentMatchesQuery(agent, "release")).toBe(true);
  expect(agentMatchesQuery(agent, "LAUNCH")).toBe(true);
  expect(agentMatchesQuery(agent, "waiting")).toBe(true);
});

it("rejects agents that do not contain the query", () => {
  expect(agentMatchesQuery(agent, "xyzzy")).toBe(false);
  expect(agentMatchesQuery({ name: "Ada", brief: "Notes" }, "review")).toBe(false);
});
