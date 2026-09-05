import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";
import type { PluginRow } from "@harbor/schema/commands";
import { ApprovalCard } from "./ApprovalCard";

interface DevicePayload {
  userCode?: string;
  verificationUri?: string;
  error?: string;
}

export function Plugins() {
  const [rows, setRows] = useState<PluginRow[]>([]);
  const [device, setDevice] = useState<DevicePayload | null>(null);
  useEffect(() => {
    void invoke<PluginRow[]>("plugin_list").then(setRows).catch(() => setRows([]));
    const unlisten = listen<DevicePayload>("plugin_device", (event) => {
      setDevice(event.payload);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);
  return (
    <Card>
      <h2>Plugins</h2>
      {rows.map((row) => (
        <div key={row.id}>
          <strong>{row.displayName}</strong> · {row.status}
          <Button onClick={() => void invoke("plugin_connect", { id: row.id })}>Connect</Button>
          {row.status === "connected" ? (
            <Button
              variant="ghost"
              onClick={() => void invoke("plugin_disconnect", { id: row.id }).then(() => invoke<PluginRow[]>("plugin_list").then(setRows))}
            >
              Disconnect
            </Button>
          ) : null}
        </div>
      ))}
      {device?.userCode ? (
        <p>
          Enter <strong>{device.userCode}</strong> at {device.verificationUri}
        </p>
      ) : null}
      {device?.error ? <p role="status">{device.error}</p> : null}
      <ApprovalCard />
    </Card>
  );
}
