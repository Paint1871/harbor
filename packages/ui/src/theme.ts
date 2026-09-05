export type Theme = "black" | "light";

/** Apply before mounting React; the host HTML must also declare data-theme="black". */
export function forceDark(root: HTMLElement = document.documentElement): void {
  applyTheme("black", root);
}

export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = theme;
  root.style.colorScheme = theme === "black" ? "dark" : "light";
}
