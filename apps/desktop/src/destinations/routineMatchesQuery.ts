export function routineMatchesQuery(
  routine: { name: string; brief: string },
  query: string,
): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  if (routine.name.toLowerCase().includes(needle)) return true;
  return routine.brief.toLowerCase().includes(needle);
}
