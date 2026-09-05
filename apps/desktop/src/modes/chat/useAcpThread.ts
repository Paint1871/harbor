import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ContentPart } from "@harbor/schema/commands";
import type { PermissionRequest } from "./PermissionCard";

export interface Line {
  id: string;
  text: string;
  role: "user" | "assistant";
}

export function useAcpThread(threadId: string | null) {
  const [lines, setLines] = useState<Line[]>([]);
  const [permission, setPermission] = useState<PermissionRequest | null>(null);
  const [configOptions, setConfigOptions] = useState<{ id: string; category: string }[]>([]);
  const [model, setModel] = useState<string | null>(null);
  const [turn, setTurn] = useState(0);

  useEffect(() => {
    let stop = () => undefined;
    void listen<{
      sessionRef: string;
      payload: { text?: string; configOptions?: { id: string; category: string }[] };
    }>("acp_update", (event) => {
      if (event.payload.sessionRef !== threadId) return;
      const text = event.payload.payload?.text;
      if (text) {
        setLines((current) => [...current, { id: `${Date.now()}-acp`, text, role: "assistant" }]);
      }
      if (event.payload.payload?.configOptions) {
        setConfigOptions(event.payload.payload.configOptions);
      }
    })
      .then((unlisten) => {
        stop = unlisten;
      })
      .catch(() => undefined);
    return () => {
      stop();
    };
  }, [threadId]);

  const send = useCallback(
    async (text: string) => {
      if (!threadId || !text.trim()) return;
      const parts: ContentPart[] = [{ type: "text", text }];
      setLines((current) => [...current, { id: `${Date.now()}`, text, role: "user" }]);
      try {
        await invoke("thread_send", { id: threadId, parts });
      } catch (error) {
        setLines((current) => [
          ...current,
          {
            id: `${Date.now()}-err`,
            role: "assistant" as const,
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
