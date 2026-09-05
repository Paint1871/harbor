export type HostPlatform = "macos" | "windows" | "linux";

export function hostPlatform(): HostPlatform {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac/i.test(ua)) return "macos";
  return "linux";
}
