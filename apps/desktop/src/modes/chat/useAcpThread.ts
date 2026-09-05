import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ContentPart } from "@harbor/schema/commands";
import type { PermissionRequest } from "./PermissionCard";

interface Line {
  id: string;
  text: string;
}

export function useAcpThread(threadId: string | null) {
  const [lines, setLines] = useState<Line[]>([]);
  const [permission, setPermission] = useState<PermissionRequest | null>(null);
  const [configOptions, setConfigOptions] = useState<{ id: string; category: string }[]>([]);
  const [model, setModel] = useState<string | null>(null);
  const [turn, setTurn] = useState(0);

  const send = useCallback(
    async (text: string) => {
      if (!threadId || !text.trim()) return;
      const parts: ContentPart[] = [{ type: "text", text }];
      setLines((current) => [...current, { id: `${Date.now()}`, text }]);
      try {
        await invoke("thread_send", { id: threadId, parts });
      } catch (error) {
        setLines((current) => [
          ...current,
          {
            id: `${Date.now()}-err`,
            text: String(error).includes("unimplemented")
              ? "Engine session starts when OpenCode ACP is connected."
              : String(error),
          },
        ]);
      }
      setTurn((value) => value + 1);
    },
    [threadId],
  );

  const resolve = useCallback(
    async (optionId: string | null, cancelled: boolean) => {
      if (!permission) return;
      try {
        await invoke("acp_permission_resolve", { id: permission.id, optionId, cancelled });
      } catch {
        /* host may still be unimplemented */
      }
      setPermission(null);
    },
    [permission],
  );

  return { lines, permission, configOptions, model, setModel, turn, send, resolve, setConfigOptions };
}
