import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";

export interface PermissionRequest {
  id: string;
  title: string;
  path?: string;
  command?: string;
  options: { optionId: string; kind: string; name: string }[];
}

const COPY: Record<string, string> = {
  allow_once: "Allow",
  allow_always: "Allow for session",
  reject_once: "Deny",
  reject_always: "Deny",
};

interface PermissionCardProps {
  request: PermissionRequest;
  onResolve: (optionId: string | null, cancelled: boolean) => void;
}

export function PermissionCard({ request, onResolve }: PermissionCardProps) {
  return (
    <Card className="harbor-permission">
      <h3>{request.title}</h3>
      {request.path ? <p>{request.path}</p> : null}
      {request.command ? <p>{request.command}</p> : null}
      <div className="harbor-welcome-actions">
        {request.options.map((option) => {
          const label = COPY[option.kind];
          if (!label) return null;
          return (
            <Button key={option.optionId} onClick={() => onResolve(option.optionId, false)}>
              {label}
            </Button>
          );
        })}
        <Button variant="ghost" onClick={() => onResolve(null, true)}>
          Stop
        </Button>
      </div>
    </Card>
  );
}
