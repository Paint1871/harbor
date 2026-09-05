async function currentWindow() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export async function minimizeWindow(): Promise<void> {
  try {
    await (await currentWindow()).minimize();
  } catch {
    /* renderer preview outside Tauri has no native window */
  }
}

export async function toggleMaximizeWindow(): Promise<void> {
  try {
    await (await currentWindow()).toggleMaximize();
  } catch {
    /* renderer preview outside Tauri has no native window */
  }
}

export async function closeWindow(): Promise<void> {
  try {
    await (await currentWindow()).close();
  } catch {
    /* renderer preview outside Tauri has no native window */
  }
}
