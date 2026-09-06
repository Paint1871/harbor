import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";

interface ApprovalCardProps {
  id?: string | null;
  onResolved?: () => void;
}

export function ApprovalCard({ id, onResolved }: ApprovalCardProps) {
  return (
    <Card>
      <span className="harbor-eyebrow">Approval</span>
      <p>Plugin writes need approval. Connection is not a grant.</p>
      {id ? (
        <div className="harbor-dialog-actions">
          <Button
            onClick={() =>
              void invoke("plugin_resolve_approval", { id, allow: true }).then(onResolved).catch(() => undefined)
            }
          >
            Allow
          </Button>
          <Button
            variant="ghost"
            onClick={() =>
              void invoke("plugin_resolve_approval", { id, allow: false }).then(onResolved).catch(() => undefined)
            }
          >
            Deny
          </Button>
        </div>
      ) : null}
    </Card>
  );
}
