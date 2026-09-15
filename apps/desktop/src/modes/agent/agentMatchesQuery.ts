export function agentMatchesQuery(
  agent: { name: string; brief: string; lastLine?: string | null },
  query: string,
): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  if (agent.name.toLowerCase().includes(needle)) return true;
  if (agent.brief.toLowerCase().includes(needle)) return true;
  return (agent.lastLine ?? "").toLowerCase().includes(needle);
}
