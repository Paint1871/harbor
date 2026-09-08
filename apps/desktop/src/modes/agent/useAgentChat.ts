import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentChat, ChatMessage, ContentPart } from "@harbor/schema/commands";
import { parsePermissionEvent, type PermissionRequest } from "../chat/PermissionCard";
import { parseConfigOptions, readConfigOptions, type AcpConfigOption } from "../chat/useAcpThread";

type AcpUpdate = {
  sessionRef: string;
  payload?: {
    text?: string;
    configOptions?: unknown;
  };
};

function asLine(id: string, role: ChatMessage["role"], text: string): ChatMessage {
  return { id, role, text };
}

export function useAgentChat(agentId: string) {
  const [chats, setChats] = useState<AgentChat[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [history, setHistory] = useState<Record<string, ChatMessage[]>>({});
  const [errors, setErrors] = useState<Record<string, string | null>>({});
  const [loading, setLoading] = useState(true);
  const [historyLoading, setHistoryLoading] = useState<Record<string, boolean>>({});
  const [sending, setSending] = useState<Record<string, boolean>>({});
  const [creating, setCreating] = useState(false);
  const [options, setOptions] = useState<Record<string, AcpConfigOption[]>>({});
  const [permissions, setPermissions] = useState<Record<string, PermissionRequest[]>>({});
  const createLock = useRef(false);
  const pending = useRef(new Set<string>());
  const requests = useRef(new Map<string, number>());
  const activeRef = useRef<string | null>(null);
  activeRef.current = activeId;

  const reloadList = useCallback(async (selectId?: string | null) => {
    setLoading(true);
    try {
      const listed = await invoke<AgentChat[]>("agent_chat_list", { agentId });
      setChats(listed);
      setActiveId((current) => {
        if (selectId && listed.some((chat) => chat.id === selectId)) return selectId;
        if (current && listed.some((chat) => chat.id === current)) return current;
        return listed[0]?.id ?? null;
      });
    } catch {
      setErrors((current) => ({
        ...current,
        [activeRef.current ?? "list"]: "Your chats could not be loaded. Please try again.",
      }));
    } finally {
      setLoading(false);
    }
  }, [agentId]);

  const loadHistory = useCallback(async (id: string) => {
    const request = (requests.current.get(id) ?? 0) + 1;
    requests.current.set(id, request);
    setHistoryLoading((current) => ({ ...current, [id]: true }));
    try {
      const lines = await invoke<ChatMessage[]>("agent_chat_history", { chatId: id });
      if (requests.current.get(id) === request) setHistory((current) => ({ ...current, [id]: lines }));
    } catch {
      if (requests.current.get(id) === request) {
        setErrors((current) => ({ ...current, [id]: "Could not load this conversation. Try again." }));
      }
    } finally {
      if (requests.current.get(id) === request) setHistoryLoading((current) => ({ ...current, [id]: false }));
    }
  }, []);

  useEffect(() => {
    setChats([]);
    setActiveId(null);
    setHistory({});
    setErrors({});
    setSending({});
    setOptions({});
    setPermissions({});
    setHistoryLoading({});
    requests.current.clear();
    void reloadList();
  }, [agentId, reloadList]);

  useEffect(() => {
    if (!activeId) return;
    void loadHistory(activeId);
  }, [activeId, loadHistory]);

  useEffect(() => {
    let disposed = false;
    const stops: Array<() => void> = [];
    void listen<AcpUpdate>("acp_update", (event) => {
      if (disposed) return;
      const sessionRef = event.payload.sessionRef;
      const text = event.payload.payload?.text?.trim() ?? "";
      const parsed = parseConfigOptions(event.payload.payload);
      if (parsed) {
        setOptions((current) => ({ ...current, [sessionRef]: parsed }));
      }
      if (text) {
        setHistory((current) => {
          const lines = current[sessionRef] ?? [];
          const last = lines[lines.length - 1];
          if (last?.role === "assistant" && last.text === text) return current;
          return {
            ...current,
            [sessionRef]: [...lines, asLine(crypto.randomUUID(), "assistant", text)],
          };
        });
      }
      setSending((current) => ({ ...current, [sessionRef]: false }));
      pending.current.delete(sessionRef);
      void invoke<AgentChat[]>("agent_chat_list", { agentId })
        .then((listed) => {
          if (!disposed) setChats(listed);
        })
        .catch(() => undefined);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stops.push(unlisten);
    }).catch(() => undefined);
    void listen<unknown>("acp_permission", (event) => {
      if (disposed) return;
      const request = parsePermissionEvent(event.payload);
      if (!request?.sessionRef) return;
      const sessionRef = request.sessionRef;
      setPermissions((current) => ({
        ...current,
        [sessionRef]: [request, ...(current[sessionRef] ?? []).filter((item) => item.id !== request.id)],
      }));
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stops.push(unlisten);
    }).catch(() => undefined);
    void listen<{ engineId?: string; hint?: string }>("engine_auth_required", (event) => {
      if (disposed) return;
      const id = activeRef.current;
      if (!id) return;
      setErrors((current) => ({
        ...current,
        [id]: `Sign in to this engine in its own app, then try again.${event.payload.hint ? ` ${event.payload.hint}` : ""}`,
      }));
      setSending((current) => ({ ...current, [id]: false }));
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stops.push(unlisten);
    }).catch(() => undefined);
    return () => {
      disposed = true;
      for (const stop of stops) stop();
    };
  }, [agentId]);

  const createChat = useCallback(async () => {
    if (createLock.current) return null;
    createLock.current = true;
    setCreating(true);
    try {
      const created = await invoke<AgentChat>("agent_chat_create", { agentId });
      setChats((current) => {
        if (current.some((chat) => chat.id === created.id)) return current;
        return [...current, created];
      });
      setActiveId(created.id);
      setHistory((current) => ({ ...current, [created.id]: current[created.id] ?? [] }));
      return created;
    } catch {
      setErrors((current) => ({
        ...current,
        [activeRef.current ?? "list"]: "The chat could not be created. Please try again.",
      }));
      return null;
    } finally {
      setCreating(false);
      createLock.current = false;
    }
  }, [agentId]);

  const send = useCallback(async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return false;
    let chatId = activeId;
    if (!chatId) {
      const created = await createChat();
      chatId = created?.id ?? null;
    }
    if (!chatId || pending.current.has(chatId)) return false;
    pending.current.add(chatId);
    requests.current.set(chatId, (requests.current.get(chatId) ?? 0) + 1);
    setHistoryLoading((current) => ({ ...current, [chatId]: false }));
    setSending((current) => ({ ...current, [chatId]: true }));
    setErrors((current) => ({ ...current, [chatId]: null }));
    setHistory((current) => ({
      ...current,
      [chatId]: [...(current[chatId] ?? []), asLine(crypto.randomUUID(), "user", trimmed)],
    }));
    const parts: ContentPart[] = [{ type: "text", text: trimmed }];
    let success = false;
    try {
      await invoke("agent_chat_send", { chatId, parts });
      success = true;
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [chatId]: `The engine could not finish this message. ${String(error)}`,
      }));
      setSending((current) => ({ ...current, [chatId]: false }));
      pending.current.delete(chatId);
    }
    void invoke<AgentChat[]>("agent_chat_list", { agentId })
      .then(setChats)
      .catch(() => undefined);
    return success;
  }, [activeId, agentId, createChat]);

  const cancel = useCallback(async () => {
    const chatId = activeId;
    if (!chatId) return;
    try {
      await invoke("agent_chat_cancel", { chatId });
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [chatId]: `Could not stop this turn. ${String(error)}`,
      }));
    } finally {
      pending.current.delete(chatId);
      setSending((current) => ({ ...current, [chatId]: false }));
    }
  }, [activeId]);

  const loadConfigOptions = useCallback(async () => {
    const chatId = activeId;
    if (!chatId) return;
    try {
      const listed = await invoke<unknown>("agent_chat_config_options", { chatId });
      setOptions((current) => ({ ...current, [chatId]: readConfigOptions(listed) }));
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [chatId]: `Could not read this engine's options. ${String(error)}`,
      }));
    }
  }, [activeId]);

  const renameChat = useCallback(async (chatId: string, title: string) => {
    const previous = chats.find((item) => item.id === chatId)?.title;
    setChats((current) => current.map((item) => item.id === chatId ? { ...item, title } : item));
    try {
      await invoke("agent_chat_rename", { chatId, title });
    } catch (error) {
      if (previous !== undefined) {
        setChats((current) => current.map((item) => item.id === chatId ? { ...item, title: previous } : item));
      }
      setErrors((current) => ({ ...current, list: `Could not rename that chat. ${String(error)}` }));
    }
  }, [chats]);

  const deleteChat = useCallback(async (chatId: string) => {
    try {
      await invoke("agent_chat_delete", { chatId });
      setChats((current) => current.filter((item) => item.id !== chatId));
      setHistory((current) => {
        const { [chatId]: _gone, ...rest } = current;
        return rest;
      });
      setActiveId((current) => current === chatId ? null : current);
    } catch (error) {
      setErrors((current) => ({ ...current, list: `Could not delete that chat. ${String(error)}` }));
    }
  }, []);

  const setConfig = useCallback(async (optionId: string, value: unknown) => {
    const chatId = activeId;
    if (!chatId) return;
    try {
      await invoke("agent_chat_set_config", { chatId, optionId, value });
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [chatId]: `Could not update engine options. ${String(error)}`,
      }));
    }
  }, [activeId]);

  const resolvePermission = useCallback(async (id: string, optionId: string | null, cancelled: boolean) => {
    const chatId = activeId;
    if (!chatId) return;
    try {
      await invoke("acp_permission_resolve", { id, optionId, cancelled });
      setPermissions((current) => ({
        ...current,
        [chatId]: (current[chatId] ?? []).filter((item) => item.id !== id),
      }));
      if (cancelled) {
        pending.current.delete(chatId);
        setSending((current) => ({ ...current, [chatId]: false }));
      }
    } catch (error) {
      setErrors((current) => ({
        ...current,
        [chatId]: `Could not resolve that permission. ${String(error)}`,
      }));
    }
  }, [activeId]);

  const chatId = activeId;
  return {
    chats,
    activeId,
    setActiveId,
    lines: chatId ? history[chatId] ?? [] : [],
    error: chatId ? errors[chatId] ?? errors.list ?? null : errors.list ?? null,
    loading,
    historyLoading: !!chatId && (historyLoading[chatId] ?? false),
    creating,
    sending: !!chatId && !!sending[chatId],
    configOptions: chatId ? options[chatId] ?? [] : [],
    permissions: chatId ? permissions[chatId] ?? [] : [],
    reload: () => {
      void reloadList();
      if (activeRef.current) void loadHistory(activeRef.current);
    },
    createChat,
    renameChat,
    deleteChat,
    send,
    cancel,
    setConfig,
    loadConfigOptions,
    resolvePermission,
  };
}
