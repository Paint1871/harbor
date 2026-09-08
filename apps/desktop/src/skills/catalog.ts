/** Local playbooks. One catalog, read by the Skills page and by an agent's Skills tab. */
export interface Skill {
  id: string;
  name: string;
  label: string;
  description: string;
  brief: string;
  tags: string[];
}

export const SKILLS: Skill[] = [
  {
    id: "review",
    name: "Code review",
    label: "Review a change with evidence",
    description: "Trace the changed paths, identify regressions, and return only findings you can support.",
    brief: "Review the current change carefully. Trace each finding to a file and line, check the tests that cover it, and finish with the smallest safe next step.",
    tags: ["Code", "Quality"],
  },
  {
    id: "release",
    name: "Release notes",
    label: "Turn work into a clear handoff",
    description: "Summarize what changed, why it matters, and what a reviewer should verify before shipping.",
    brief: "Draft release notes from the current work. Separate user-visible changes, fixes, and follow-ups. Keep the language concrete and flag anything that still needs verification.",
    tags: ["Writing", "Handoff"],
  },
  {
    id: "research",
    name: "Research brief",
    label: "Make a decision easier",
    description: "Collect the relevant context, compare the options, and end with a concise recommendation.",
    brief: "Prepare a short research brief for this question. Start with the decision, list the strongest evidence and unknowns, compare the options, and recommend a next step.",
    tags: ["Research", "Decisions"],
  },
];
