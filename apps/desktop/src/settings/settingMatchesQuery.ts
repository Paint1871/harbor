export function settingMatchesQuery(label: string, description: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  if (label.toLowerCase().includes(needle)) return true;
  return description.toLowerCase().includes(needle);
}
