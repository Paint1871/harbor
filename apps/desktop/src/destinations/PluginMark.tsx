const MARKS: Record<string, string> = {
  github: "GH",
  linear: "L",
  slack: "S",
  notion: "N",
  x: "𝕏",
  apollo: "A",
  vidiq: "V",
  higgsfield: "H",
  fal: "F",
  youtube: "▶",
  stripe: "S",
  cloudflare: "C",
  gmail: "M",
  supabase: "S",
  vercel: "▲",
  shopify: "S",
};

export function PluginMark({ id }: { id: string }) {
  return (
    <span className={`harbor-plugin-mark harbor-plugin-mark-${id}`} aria-hidden="true">
      <span>{MARKS[id] ?? id.slice(0, 2).toUpperCase()}</span>
    </span>
  );
}
