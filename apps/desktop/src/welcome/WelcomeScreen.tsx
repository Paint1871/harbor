import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Logo } from "@harbor/ui/Logo";
import { useTheme } from "@harbor/ui/ThemeProvider";
import { Orbit } from "./Orbit";

export interface WelcomeScreenProps {
  onStartLocal: (profileName: string) => void | Promise<void>;
}

const TAGLINE = ["An open desktop host", "for coding agents"] as const;

function RisingLine({ text, offset }: { text: string; offset: number }) {
  return (
    <span className="harbor-welcome-line">
      {Array.from(text).map((glyph, index) => (
        <span key={`${glyph}-${index}`} style={{ "--d": offset + index } as CSSProperties}>
          {glyph === " " ? "\u00a0" : glyph}
        </span>
      ))}
    </span>
  );
}

export function WelcomeScreen({ onStartLocal }: WelcomeScreenProps) {
  const { reducedMotion } = useTheme();
  const [profileOpen, setProfileOpen] = useState(false);
  const [name, setName] = useState("Local");
  const [busy, setBusy] = useState(false);

  // The OS account is a suggestion; nothing is stored until Start local.
  useEffect(() => {
    let cancelled = false;
    void invoke<string>("default_profile_name")
      .then((suggested) => {
        if (!cancelled && suggested.trim()) setName(suggested.trim());
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  async function start(profileName: string) {
    setBusy(true);
    try {
      await onStartLocal(profileName.trim() || "Local");
    } finally {
      setBusy(false);
    }
  }

  const valid = name.trim().length >= 1 && name.trim().length <= 40;

  return (
    <main
      className="harbor-welcome"
      data-still={reducedMotion || undefined}
      data-dialog={profileOpen || undefined}
    >
      <Orbit />
      <div className="harbor-welcome-mark">
        <Logo size={28} />
        <h1 className="harbor-welcome-wordmark">Harbor</h1>
      </div>
      <hr className="harbor-welcome-rule" />
      <p className="harbor-welcome-tagline" aria-label="An open desktop host for coding agents">
        <RisingLine text={TAGLINE[0]} offset={0} />
        <RisingLine text={TAGLINE[1]} offset={TAGLINE[0].length + 4} />
      </p>
      <div className="harbor-welcome-actions">
        <Button variant="primary" disabled={busy} onClick={() => void start(name)}>
          Start local
        </Button>
        <Button disabled={busy} onClick={() => setProfileOpen(true)}>
          Local profile
        </Button>
      </div>
      {profileOpen ? (
        <div className="harbor-dialog" role="dialog" aria-labelledby="profile-title">
          <h2 id="profile-title">Local profile</h2>
          <p>A name on this machine. Harbor does not create an account.</p>
          <label>
            Name
            <input
              value={name}
              maxLength={40}
              onChange={(event) => setName(event.target.value)}
              autoFocus
            />
          </label>
          <div className="harbor-welcome-actions">
            <Button variant="primary" disabled={busy || !valid} onClick={() => void start(name)}>
              Start local
            </Button>
            <Button disabled={busy} onClick={() => setProfileOpen(false)}>
              Cancel
            </Button>
          </div>
        </div>
      ) : null}
    </main>
  );
}
