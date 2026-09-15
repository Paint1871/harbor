/** Play only for live events while Harbor is in the background and the setting is on. */
export function shouldPlayNotificationSound(hidden: boolean, soundEnabled: unknown): boolean {
  return hidden && soundEnabled === true;
}

/**
 * A short, quiet original blip: one sine, fast decay, no sample file.
 * Web Audio is optional; a missing or blocked context is not a product error.
 */
export function playNotificationBeep(): void {
  const Ctor = window.AudioContext;
  if (typeof Ctor !== "function") return;
  try {
    const ctx = new Ctor();
    const oscillator = ctx.createOscillator();
    const gain = ctx.createGain();
    oscillator.type = "sine";
    oscillator.frequency.value = 704;
    gain.gain.value = 0;
    oscillator.connect(gain);
    gain.connect(ctx.destination);
    const now = ctx.currentTime;
    gain.gain.setValueAtTime(0.035, now);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.09);
    oscillator.start(now);
    oscillator.stop(now + 0.1);
    oscillator.onended = () => {
      void ctx.close();
    };
  } catch {
    /* autoplay policy or a stubbed AudioContext */
  }
}
