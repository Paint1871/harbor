export function memoryMatchesQuery(body: string, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return body.toLowerCase().includes(needle);
}
