import { Button } from "@harbor/ui/Button";
import { Card } from "@harbor/ui/Card";

export interface PermissionRequest {
  id: string;
  title: string;
  path?: string;
  command?: string;
  options: { optionId: string; kind: string; name: string }[];
  sessionRef?: string;
}

export function parsePermissionEvent(payload: unknown): PermissionRequest | null {
  if (!payload || typeof payload !== "object") return null;
  const data = payload as Record<string, unknown>;
  if (typeof data.id !== "string" || !data.id || typeof data.title !== "string") return null;
  const options = Array.isArray(data.options)
    ? data.options.flatMap((item) => {
        if (!item || typeof item !== "object") return [];
        const option = item as Record<string, unknown>;
        if (typeof option.optionId !== "string" || typeof option.kind !== "string") return [];
        return [{
          optionId: option.optionId,
          kind: option.kind,
          name: typeof option.name === "string" ? option.name : "",
        }];
      })
    : [];
  return {
    id: data.id,
    title: data.title,
    path: typeof data.path === "string" ? data.path : undefined,
    command: typeof data.command === "string" ? data.command : undefined,
    options,
    sessionRef: typeof data.sessionRef === "string" ? data.sessionRef : undefined,
  };
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
