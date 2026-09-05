import { invoke } from "@tauri-apps/api/core";

export async function settingsGet(key: string): Promise<unknown> {
  try {
    return await invoke("settings_get", { key });
  } catch {
    return null;
  }
}

export async function settingsSet(key: string, value: unknown): Promise<void> {
  try {
    await invoke("settings_set", { key, value });
  } catch {
    /* vite preview outside the native host */
  }
}
