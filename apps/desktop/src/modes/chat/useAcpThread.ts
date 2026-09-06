import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage, ContentPart } from "@harbor/schema/commands";
import { parsePermissionEvent, type PermissionRequest } from "./PermissionCard";

export interface AcpConfigOption {
  id: string;
  category: string;
}

export function parseConfigOptions(payload: unknown): AcpConfigOption[] | null {
  if (!payload || typeof payload !== "object") return null;
  const raw = (payload as { configOptions?: unknown }).configOptions;
  if (!Array.isArray(raw)) return null;
  const options: AcpConfigOption[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const id = (item as { id?: unknown }).id;
    if (typeof id !== "string" || !id.trim()) continue;
    const category = (item as { category?: unknown }).category;
    options.push({
      id,
      category: typeof category === "string" && category.trim() ? category : "model",
    });
  }
  return options;
}

export function useAcpThread(threadId: string | null) {
  const [history, setHistory] = useState<Record<string, ChatMessage[]>>({});
  const [errors, setErrors] = useState<Record<string, string | null>>({});
  const [loading, setLoading] = useState<Record<string, boolean>>({});
  const [sending, setSending] = useState<Record<string, boolean>>({});
  const [options, setOptions] = useState<Record<string, AcpConfigOption[]>>({});
  const [permissions, setPermissions] = useState<Record<string, PermissionRequest[]>>({});
  const [turn, setTurn] = useState(0);
  const pending = useRef(new Set<string>());
  const requests = useRef(new Map<string, number>());

  const reload = useCallback(async (id: string) => {
    const request = (requests.current.get(id) ?? 0) + 1;
    requests.current.set(id, request);
    setLoading((current) => ({ ...current, [id]: true }));
    try {
      const lines = await invoke<ChatMessage[]>("thread_history", { id });
      if (requests.current.get(id) === request) setHistory((current) => ({ ...current, [id]: lines }));
    } catch {
      if (requests.current.get(id) === request) setErrors((current) => ({ ...current, [id]: "Could not load this conversation. Try again." }));
    } finally {
      if (requests.current.get(id) === request) setLoading((current) => ({ ...current, [id]: false }));
    }
  }, []);

  useEffect(() => {
    if (!threadId) return;
    void reload(threadId);
    let disposed = false;
    const stops: Array<() => void> = [];
    void listen<{ sessionRef: string; payload?: unknown }>("acp_update", (event) => {
      if (disposed || event.payload.sessionRef !== threadId) return;
      const parsed = parseConfigOptions(event.payload.payload);
      if (parsed) setOptions((current) => ({ ...current, [threadId]: parsed }));
      void reload(threadId);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stops.push(unlisten);
    }).catch(() => undefined);
    void listen<unknown>("acp_permission", (event) => {
      if (disposed) return;
      const request = parsePermissionEvent(event.payload);
      if (!request || request.sessionRef !== threadId) return;
      setPermissions((current) => ({
        ...current,
        [threadId]: [request, ...(current[threadId] ?? []).filter((item) => item.id !== request.id)],
      }));
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stops.push(unlisten);
    }).catch(() => undefined);
    return () => { disposed = true; for (const unlisten of stops) unlisten(); };
  }, [reload, threadId]);

  const send = useCallback(async (text: string) => {
    if (!threadId || !text.trim() || pending.current.has(threadId)) return false;
    const id = threadId;
    pending.current.add(id);
    setSending((current) => ({ ...current, [id]: true }));
    setErrors((current) => ({ ...current, [id]: null }));
    // Invalidate an older history read before showing the optimistic message.
    requests.current.set(id, (requests.current.get(id) ?? 0) + 1);
    setLoading((current) => ({ ...current, [id]: false }));
    setHistory((current) => ({ ...current, [id]: [...(current[id] ?? []), { id: crypto.randomUUID(), role: "user", text }] }));
    const parts: ContentPart[] = [{ type: "text", text }];
    let success = false;
    try {
      await invoke("thread_send", { id, parts });
      success = true;
    } catch (error) {
      setErrors((current) => ({ ...current, [id]: `The engine could not finish this message. ${String(error)}` }));
    } finally {
      await reload(id);
      pending.current.delete(id);
      setSending((current) => ({ ...current, [id]: false }));
      setTurn((value) => value + 1);
    }
    return success;
  }, [reload, threadId]);

  const cancel = useCallback(async () => {
    if (!threadId) return;
    const id = threadId;
    try {
      await invoke("thread_cancel", { id });
    } catch (error) {
      setErrors((current) => ({ ...current, [id]: `Could not stop this turn. ${String(error)}` }));
    } finally {
      pending.current.delete(id);
      setSending((current) => ({ ...current, [id]: false }));
    }
  }, [threadId]);

  const resolvePermission = useCallback(async (id: string, optionId: string | null, cancelled: boolean) => {
    const session = threadId;
    if (!session) return;
    try {
      await invoke("acp_permission_resolve", { id, optionId, cancelled });
      setPermissions((current) => ({
        ...current,
        [session]: (current[session] ?? []).filter((item) => item.id !== id),
      }));
      if (cancelled) {
        pending.current.delete(session);
        setSending((current) => ({ ...current, [session]: false }));
      }
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [session]: `Could not resolve that permission. ${String(error)}`,
      }));
    }
  }, [threadId]);

  return {
    lines: threadId ? history[threadId] ?? [] : [],
    error: threadId ? errors[threadId] : null,
    loading: !!threadId && (loading[threadId] ?? !(threadId in history)),
    sending: !!threadId && !!sending[threadId],
    configOptions: threadId ? options[threadId] ?? [] : [],
    permissions: threadId ? permissions[threadId] ?? [] : [],
    turn,
    send,
    cancel,
    resolvePermission,
    reload: () => {
      if (threadId) {
        setErrors((current) => ({ ...current, [threadId]: null }));
        void reload(threadId);
      }
    },
  };
}
