import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";
import type { PluginRow } from "@harbor/schema/commands";
import { ApprovalCard } from "./ApprovalCard";

export function Plugins() {
  const [rows, setRows] = useState<PluginRow[]>([]);
  useEffect(() => {
    void invoke<PluginRow[]>("plugin_list").then(setRows).catch(() => setRows([]));
  }, []);
  return (
    <Card>
      <h2>Plugins</h2>
      {rows.map((row) => (
        <div key={row.id}>
          <strong>{row.displayName}</strong> · {row.status}
          <Button onClick={() => void invoke("plugin_connect", { id: row.id })}>Connect</Button>
        </div>
      ))}
      <ApprovalCard />
    </Card>
  );
}
