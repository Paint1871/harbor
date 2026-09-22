import { useState } from "react";
import { call } from "../ipc";
import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";
import type { PluginApproval } from "@harbor/schema/commands";

interface ApprovalCardProps {
  approval: PluginApproval;
  agentName?: string;
  pluginName?: string;
  onResolved?: () => void;
}

export function ApprovalCard({ approval, agentName, pluginName, onResolved }: ApprovalCardProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const id = approval.id;
  const agent = agentName ?? approval.agentId ?? "A teammate";
  const plugin = pluginName ?? approval.pluginId;
  const request =
    approval.action === "grant"
      ? `${agent} asks to use ${plugin}. Allowing grants ${plugin} to this teammate; the session picks it up without a restart.`
      : `${agent} asks for ${plugin}: ${approval.action}.`;
  function resolve(allow: boolean) {
    if (busy) return;
    setBusy(true);
    setError(null);
    void call("plugin_resolve_approval", { id, allow })
      .then(() => onResolved?.())
      // A failed resolution keeps the row pending; say so instead of letting
      // the card look decided while nothing changed.
      .catch((reason) => setError(`The approval could not be applied. ${String(reason)}`))
      .finally(() => setBusy(false));
  }
  return (
    <Card>
      <span className="harbor-eyebrow">Approval</span>
      <p>{request} Connection is not a grant.</p>
      {error ? <p className="harbor-inline-error" role="alert">{error}</p> : null}
      {id ? (
        <div className="harbor-dialog-actions">
          <Button disabled={busy} onClick={() => resolve(true)}>Allow</Button>
          <Button variant="ghost" disabled={busy} onClick={() => resolve(false)}>Deny</Button>
        </div>
      ) : null}
    </Card>
  );
}
