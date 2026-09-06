import { RailRow } from "@harbor/ui/RailRow";
import { useChrome, type Destination } from "./chrome-context";

const ROWS: { id: Exclude<Destination, "mode">; label: string; trailing?: string }[] = [
  { id: "dashboard", label: "Dashboard", trailing: "1" },
  { id: "routines", label: "Routines" },
  { id: "plugins", label: "Plugins" },
  { id: "skills", label: "Skills" },
];

export function Destinations() {
  const { destination, setDestination } = useChrome();
  return (
    <nav className="harbor-destinations" aria-label="Destinations">
      {ROWS.map((row) => (
        <RailRow
          key={row.id}
          label={row.label}
          trailing={row.trailing ? <span className="harbor-destination-badge">{row.trailing}</span> : undefined}
          selected={destination === row.id}
          onClick={() => setDestination(row.id)}
        />
      ))}
    </nav>
  );
}
