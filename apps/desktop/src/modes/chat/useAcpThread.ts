import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage, ContentPart } from "@harbor/schema/commands";
import { parsePermissionEvent, type PermissionRequest } from "./PermissionCard";

export interface AcpConfigValue {
  value: string;
  name: string;
  description?: string | null;
}

/** One knob the engine exposes — "Model", "Session Mode" — and the values it takes. */
export interface AcpConfigOption {
  id: string;
  name: string;
  category: string;
  currentValue: string | null;
  values: AcpConfigValue[];
  /** False where the engine reports the value but offers no way to change it. */
  settable: boolean;
}

function readString(source: object, key: string): string | null {
  const value = (source as Record<string, unknown>)[key];
  return typeof value === "string" && value.trim() ? value : null;
}

function readValues(source: object): AcpConfigValue[] {
  const raw = (source as { values?: unknown }).values;
  if (!Array.isArray(raw)) return [];
  const values: AcpConfigValue[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const value = readString(item, "value");
    if (!value) continue;
    values.push({ value, name: readString(item, "name") ?? value, description: readString(item, "description") });
  }
  return values;
}

export function readConfigOptions(raw: unknown): AcpConfigOption[] {
  if (!Array.isArray(raw)) return [];
  const options: AcpConfigOption[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const id = readString(item, "id");
    if (!id) continue;
    options.push({
      id,
      name: readString(item, "name") ?? id,
      category: readString(item, "category") ?? "model",
      currentValue: readString(item, "currentValue"),
      values: readValues(item),
      settable: (item as { settable?: unknown }).settable !== false,
    });
  }
  return options;
}

export function parseConfigOptions(payload: unknown): AcpConfigOption[] | null {
  if (!payload || typeof payload !== "object") return null;
  const raw = (payload as { configOptions?: unknown }).configOptions;
  if (!Array.isArray(raw)) return null;
  return readConfigOptions(raw);
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

  /** Options belong to one engine's session. Switching engines invalidates them. */
  const resetConfigOptions = useCallback(() => {
    if (!threadId) return;
    setOptions((current) => ({ ...current, [threadId]: [] }));
  }, [threadId]);

  /** Opening the picker is the moment to ask; connecting an engine is not free. */
  const loadConfigOptions = useCallback(async () => {
    if (!threadId) return;
    const id = threadId;
    try {
      const listed = await invoke<unknown>("thread_config_options", { id });
      setOptions((current) => ({ ...current, [id]: readConfigOptions(listed) }));
    } catch (error) {
      setErrors((current) => ({ ...current, [id]: `Could not read this engine's options. ${String(error)}` }));
    }
  }, [threadId]);

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
    loadConfigOptions,
    resetConfigOptions,
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
