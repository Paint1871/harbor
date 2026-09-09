import { call } from "./ipc";

export async function settingsGet(key: string): Promise<unknown> {
  try {
    return await call("settings_get", { key });
  } catch {
    return null;
  }
}

export async function settingsSet(key: string, value: unknown): Promise<void> {
  try {
    await call("settings_set", { key, value });
  } catch {
    /* vite preview outside the native host */
  }
}
