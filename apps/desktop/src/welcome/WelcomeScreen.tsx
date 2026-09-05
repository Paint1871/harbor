import { useState } from "react";
import { Button } from "@harbor/ui/Button";
import { Logo } from "@harbor/ui/Logo";
import { Orbit } from "./Orbit";

export interface WelcomeScreenProps {
  onStartLocal: (profileName: string) => void | Promise<void>;
}

export function WelcomeScreen({ onStartLocal }: WelcomeScreenProps) {
  const [profileOpen, setProfileOpen] = useState(false);
  const [name, setName] = useState("Builder");
  const [busy, setBusy] = useState(false);

  async function start(profileName: string) {
    setBusy(true);
    try {
      await onStartLocal(profileName.trim() || "Builder");
    } finally {
      setBusy(false);
    }
  }

  const valid = name.trim().length >= 1 && name.trim().length <= 40;

  return (
    <main className="harbor-welcome">
      <Orbit />
      <div className="harbor-welcome-mark">
        <Logo size={28} />
        <h1>Harbor</h1>
      </div>
      <p className="harbor-welcome-tagline">An open desktop host for coding agents</p>
      <div className="harbor-welcome-actions">
        <Button variant="primary" disabled={busy} onClick={() => void start("Builder")}>
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
