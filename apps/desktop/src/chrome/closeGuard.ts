import { hasUnsavedBuffers, saveUnsavedBuffers } from "../panes/files/dirtyFiles";

const CLOSE_SAVE_FAILED = "Some files could not be saved. Close Harbor anyway?";
const FLUSH_TIMEOUT_MS = 2000;

/**
 * Keep an app close from dropping dirty editor buffers. The native close
 * request is held back while buffers flush; a failed write asks the user
 * instead of silently losing the file. Outside Tauri (preview/tests) the
 * browser beforeunload prompt covers the same case.
 */
export function installCloseGuard(): void {
  if (typeof window === "undefined") return;
  window.addEventListener("beforeunload", (event) => {
    if (hasUnsavedBuffers()) event.preventDefault();
  });
  if (!("__TAURI_INTERNALS__" in window)) return;
  void import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => {
      const appWindow = getCurrentWindow();
      // close() re-fires close-requested; without this flag a successful
      // flush would loop back into the guard instead of closing.
      let closing = false;
      void appWindow.onCloseRequested((event) => {
        if (closing || !hasUnsavedBuffers()) return;
        event.preventDefault();
        void saveUnsavedBuffers(FLUSH_TIMEOUT_MS).then((failed) => {
          if (failed.length && !window.confirm(CLOSE_SAVE_FAILED)) return;
          closing = true;
          void appWindow.close();
        });
      });
    })
    .catch(() => undefined);
}
