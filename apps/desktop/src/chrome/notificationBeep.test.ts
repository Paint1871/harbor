// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { playNotificationBeep, shouldPlayNotificationSound } from "./notificationBeep";

describe("shouldPlayNotificationSound", () => {
  it("only plays in the background when the setting is on", () => {
    expect(shouldPlayNotificationSound(true, true)).toBe(true);
    expect(shouldPlayNotificationSound(false, true)).toBe(false);
    expect(shouldPlayNotificationSound(true, false)).toBe(false);
    expect(shouldPlayNotificationSound(true, null)).toBe(false);
    expect(shouldPlayNotificationSound(false, false)).toBe(false);
  });
});

describe("playNotificationBeep", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("does not throw when Web Audio is missing", () => {
    vi.stubGlobal("AudioContext", undefined);
    expect(() => playNotificationBeep()).not.toThrow();
  });

  it("starts a quiet short oscillator", () => {
    const start = vi.fn();
    const stop = vi.fn();
    const connect = vi.fn();
    const setValueAtTime = vi.fn();
    const exponentialRampToValueAtTime = vi.fn();
    const close = vi.fn().mockResolvedValue(undefined);
    class FakeAudioContext {
      currentTime = 1.5;
      destination = {};
      createOscillator() {
        return {
          type: "sine" as OscillatorType,
          frequency: { value: 0 },
          connect,
          start,
          stop,
          onended: null as (() => void) | null,
        };
      }
      createGain() {
        return {
          gain: { value: 0, setValueAtTime, exponentialRampToValueAtTime },
          connect,
        };
      }
      close = close;
    }
    vi.stubGlobal("AudioContext", FakeAudioContext);
    expect(() => playNotificationBeep()).not.toThrow();
    expect(start).toHaveBeenCalledWith(1.5);
    expect(stop).toHaveBeenCalledWith(1.6);
    expect(setValueAtTime.mock.calls[0]?.[0]).toBeLessThan(0.1);
    expect(exponentialRampToValueAtTime).toHaveBeenCalled();
  });
});
