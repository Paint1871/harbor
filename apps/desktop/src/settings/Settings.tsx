import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@harbor/ui/Button";
import { Segmented } from "@harbor/ui/Segmented";
import type { DetectedEngine } from "@harbor/schema/commands";
import type { Theme } from "@harbor/ui/theme";
import { settingsGet, settingsSet } from "./api";

const PAGES = [
  { value: "general", label: "General" },
  { value: "notifications", label: "Notifications" },
  { value: "voice", label: "Voice" },
  { value: "agents", label: "Agents" },
  { value: "account", label: "Account" },
] as const;

type Page = (typeof PAGES)[number]["value"];

export interface SettingsProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  initialPage?: Page;
  profileName?: string;
  onProfileNameChange?: (value: string) => void;
  reduceMotion?: boolean;
  onReduceMotionChange?: (value: boolean) => void;
  onClose: () => void;
  onShowWelcome: () => void;
}

interface SettingRowProps {
  label: string;
  description: string;
  children: ReactNode;
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="harbor-setting-row">
      <div className="harbor-setting-copy">
        <strong>{label}</strong>
        <span>{description}</span>
      </div>
      <div className="harbor-setting-control">{children}</div>
    </div>
  );
}

function Toggle({ label, description, checked, onChange }: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="harbor-toggle-row">
      <div className="harbor-setting-copy">
        <strong>{label}</strong>
        <span>{description}</span>
      </div>
      <button
        type="button"
        role="switch"
        aria-label={label}
        aria-checked={checked}
        className="harbor-switch"
        data-checked={checked}
        onClick={() => onChange(!checked)}
      >
        <span aria-hidden="true" />
      </button>
    </div>
  );
}

function Select({ label, value, options, onChange, disabled = false }: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <select aria-label={label} value={value} disabled={disabled} onChange={(event) => onChange(event.target.value)}>
      {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
    </select>
  );
}

function valueString(value: unknown, fallback: string): string {
  return typeof value === "string" && value ? value : fallback;
}

function valueBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

export function Settings({
  theme,
  onThemeChange,
  initialPage = "general",
  profileName = "Local",
  onProfileNameChange,
  reduceMotion = false,
  onReduceMotionChange,
  onClose,
  onShowWelcome,
}: SettingsProps) {
  const [page, setPage] = useState<Page>(initialPage);
  const [uiZoom, setUiZoom] = useState("100");
  const [startupMode, setStartupMode] = useState("last");
  const [defaultShell, setDefaultShell] = useState("system");
  const [notifications, setNotifications] = useState(true);
  const [notificationSound, setNotificationSound] = useState(false);
  const [voiceHandsFree, setVoiceHandsFree] = useState(false);
  const [speechMode, setSpeechMode] = useState("on-device");
  const [voiceUrl, setVoiceUrl] = useState("");
  const [mailPaused, setMailPaused] = useState(false);
  const [memoryDefault, setMemoryDefault] = useState("facts");
  const [defaultEngine, setDefaultEngine] = useState("auto");
  const [accountName, setAccountName] = useState(profileName);
  const [engines, setEngines] = useState<DetectedEngine[]>([]);
  const [rechecking, setRechecking] = useState(false);
  const [engineStatus, setEngineStatus] = useState<string | null>(null);
  const [voiceStatus, setVoiceStatus] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void Promise.all([
      settingsGet("ui_zoom"),
      settingsGet("startup_mode"),
      settingsGet("default_shell"),
      settingsGet("notifications_enabled"),
      settingsGet("notification_sound"),
      settingsGet("voice_hands_free"),
      settingsGet("speech_mode"),
      settingsGet("voice_cloud_url"),
      settingsGet("agent_mail_paused"),
      settingsGet("memory_default"),
      settingsGet("default_engine"),
      settingsGet("local_profile_name"),
    ]).then(([zoom, startup, shell, notify, sound, handsFree, speech, cloudUrl, paused, memory, engine, name]) => {
      if (!active) return;
      setUiZoom(valueString(zoom, "100"));
      setStartupMode(valueString(startup, "last"));
      setDefaultShell(valueString(shell, "system"));
      setNotifications(valueBoolean(notify, true));
      setNotificationSound(valueBoolean(sound, false));
      setVoiceHandsFree(valueBoolean(handsFree, false));
      setSpeechMode(valueString(speech, "on-device"));
      setVoiceUrl(valueString(cloudUrl, ""));
      setMailPaused(valueBoolean(paused, false));
      setMemoryDefault(valueString(memory, "facts"));
      setDefaultEngine(valueString(engine, "auto"));
      setAccountName(valueString(name, profileName));
    });
    void invoke<DetectedEngine[]>("engines_detect").then(setEngines).catch(() => setEngines([]));
    return () => { active = false; };
  }, [profileName]);

  async function save(key: string, value: unknown) {
    await settingsSet(key, value);
    setSaved("Saved locally");
    window.setTimeout(() => setSaved(null), 1800);
  }

  async function recheckEngines() {
    setRechecking(true);
    setEngineStatus(null);
    try {
      const next = await invoke<DetectedEngine[]>("engines_recheck");
      setEngines(next);
      setEngineStatus(next.filter((engine) => engine.status === "ready").length ? "Engines ready" : "No ready engines found");
    } catch {
      setEngineStatus("Engine check is available in the Harbor desktop host.");
    } finally {
      setRechecking(false);
    }
  }

  async function voiceAction(command: "dictation_begin" | "dictation_end" | "dictation_prepare_model", success: string) {
    setVoiceStatus(null);
    try {
      await invoke(command);
      setVoiceStatus(success);
    } catch {
      setVoiceStatus("Voice is not available in this build yet. Dictation remains local to the desktop host.");
    }
  }

  function saveProfile() {
    const next = accountName.trim() || "Local";
    setAccountName(next);
    onProfileNameChange?.(next);
    void save("local_profile_name", next);
  }

  return (
    <div className="harbor-settings" role="dialog" aria-label="Settings">
      <header>
        <div className="harbor-settings-title">
          <span className="harbor-eyebrow">HARBOR</span>
          <h2>Settings</h2>
        </div>
        <div className="harbor-settings-header-actions">
          {saved ? <span className="harbor-settings-saved" role="status">{saved}</span> : null}
          <Button variant="ghost" onClick={onClose}>Close</Button>
        </div>
      </header>
      <div className="harbor-settings-body">
        <nav aria-label="Settings sections">
          {PAGES.map((item) => (
            <Button key={item.value} variant={page === item.value ? "primary" : "ghost"} onClick={() => setPage(item.value)}>
              {item.label}
            </Button>
          ))}
        </nav>
        <section className="harbor-settings-content">
          {page === "general" ? (
            <>
              <div className="harbor-settings-intro">
                <span className="harbor-eyebrow">WORKSPACE PREFERENCES</span>
                <h3>General</h3>
                <p>Make Harbor feel like your desk. Every preference is stored on this machine.</p>
              </div>
              <div className="harbor-settings-list">
                <SettingRow label="Appearance" description="Choose the calm, dark-first canvas or a light paper surface.">
                  <Segmented label="Appearance" value={theme} options={[{ value: "black", label: "Black" }, { value: "light", label: "Light" }]} onValueChange={(value) => { onThemeChange(value); void save("appearance", value); }} />
                </SettingRow>
                <SettingRow label="UI zoom" description="Adjust the reading size without changing your display settings.">
                  <Select label="UI zoom" value={uiZoom} options={["90", "100", "110", "125"].map((value) => ({ value, label: `${value}%` }))} onChange={(value) => { setUiZoom(value); void save("ui_zoom", value); }} />
                </SettingRow>
                <SettingRow label="Startup" description="Choose where Harbor opens after you launch it.">
                  <Select label="Startup" value={startupMode} options={[{ value: "last", label: "Last workspace" }, { value: "welcome", label: "Welcome" }, { value: "agent", label: "Agent mode" }]} onChange={(value) => { setStartupMode(value); void save("startup_mode", value); }} />
                </SettingRow>
                <SettingRow label="Language" description="The first release ships in English. More languages can follow later.">
                  <Select label="Language" value="english" options={[{ value: "english", label: "English" }]} onChange={() => undefined} disabled />
                </SettingRow>
                <SettingRow label="Default shell" description="The shell used when a new terminal pane starts.">
                  <Select label="Default shell" value={defaultShell} options={[{ value: "system", label: "System default" }, { value: "zsh", label: "zsh" }, { value: "bash", label: "bash" }]} onChange={(value) => { setDefaultShell(value); void save("default_shell", value); }} />
                </SettingRow>
                <SettingRow label="Engine discovery" description={engineStatus ?? "Refresh the local engine list before starting a session."}>
                  <Button variant="ghost" disabled={rechecking} onClick={() => void recheckEngines()}>{rechecking ? "Checking…" : "Recheck engines"}</Button>
                </SettingRow>
                <Toggle label="Reduce motion" description="Use still transitions and avoid animated halos." checked={reduceMotion} onChange={(value) => { onReduceMotionChange?.(value); void save("reduce_motion", value); }} />
              </div>
            </>
          ) : null}

          {page === "notifications" ? (
            <>
              <div className="harbor-settings-intro">
                <span className="harbor-eyebrow">SIGNALS</span>
                <h3>Notifications</h3>
                <p>Harbor keeps notification text short: a state change, not a transcript in disguise.</p>
              </div>
              <div className="harbor-settings-list">
                <Toggle label="Desktop notifications" description="Show prompt, permission, question, complete, and fail events." checked={notifications} onChange={(value) => { setNotifications(value); void save("notifications_enabled", value); }} />
                <Toggle label="Notification sound" description="Play a quiet sound for events while Harbor is in the background." checked={notificationSound} onChange={(value) => { setNotificationSound(value); void save("notification_sound", value); }} />
              </div>
              <div className="harbor-settings-callout"><span className="harbor-live-dot" data-on="true" /><span><strong>Event types</strong><small>Prompt · Permission · Question · Complete · Fail</small></span></div>
            </>
          ) : null}

          {page === "voice" ? (
            <>
              <div className="harbor-settings-intro">
                <span className="harbor-eyebrow">OPTIONAL INPUT</span>
                <h3>Voice</h3>
                <p>Dictation stays on-device by default. Cloud speech is opt-in and only uses a URL you provide.</p>
              </div>
              <div className="harbor-settings-list">
                <SettingRow label="Speech recognition" description="Pick the local path or your own compatible endpoint.">
                  <Segmented label="Speech recognition" value={speechMode} options={[{ value: "on-device", label: "On-device" }, { value: "cloud", label: "Your URL" }]} onValueChange={(value) => { setSpeechMode(value); void save("speech_mode", value); }} />
                </SettingRow>
                {speechMode === "cloud" ? <SettingRow label="Cloud URL" description="Harbor does not provide a hosted speech service."><input aria-label="Cloud URL" value={voiceUrl} placeholder="https://your-endpoint.example/transcribe" onChange={(event) => setVoiceUrl(event.target.value)} onBlur={() => void save("voice_cloud_url", voiceUrl.trim())} /></SettingRow> : null}
                <Toggle label="Hands-free mode" description="Keep listening after a dictation turn. Off by default." checked={voiceHandsFree} onChange={(value) => { setVoiceHandsFree(value); void save("voice_hands_free", value); }} />
              </div>
              <div className="harbor-settings-actions">
                <Button onClick={() => void voiceAction("dictation_begin", "Listening for a short test…")}>Test dictation</Button>
                <Button variant="ghost" onClick={() => void voiceAction("dictation_end", "Dictation stopped.")}>Stop</Button>
                <Button variant="ghost" onClick={() => void voiceAction("dictation_prepare_model", "Local model prepared.")}>Prepare Whisper model</Button>
              </div>
              {voiceStatus ? <p className="harbor-settings-status" role="status">{voiceStatus}</p> : null}
            </>
          ) : null}

          {page === "agents" ? (
            <>
              <div className="harbor-settings-intro">
                <span className="harbor-eyebrow">LOCAL TEAMWORK</span>
                <h3>Agents</h3>
                <p>Set the defaults shared by your local teammates. Individual agents can still override their own engine and access.</p>
              </div>
              <div className="harbor-settings-list">
                <Toggle label="Pause agent mail" description="Temporarily stop handoffs between agents without deleting them." checked={mailPaused} onChange={(value) => { setMailPaused(value); void save("agent_mail_paused", value); }} />
                <SettingRow label="Default engine" description="Used when a new agent does not have a specific engine yet.">
                  <Select label="Default engine" value={defaultEngine} options={[{ value: "auto", label: "Auto-detect" }, ...engines.filter((engine) => engine.status === "ready" && engine.supportsChat).map((engine) => ({ value: engine.id, label: engine.displayName }))]} onChange={(value) => { setDefaultEngine(value); void save("default_engine", value); }} />
                </SettingRow>
                <SettingRow label="Memory defaults" description="New memories are facts, never credentials or secrets.">
                  <Select label="Memory defaults" value={memoryDefault} options={[{ value: "facts", label: "Facts only" }, { value: "off", label: "Ask every time" }]} onChange={(value) => { setMemoryDefault(value); void save("memory_default", value); }} />
                </SettingRow>
              </div>
              <div className="harbor-settings-callout"><span className="harbor-chip">{engines.filter((engine) => engine.status === "ready").length} ready</span><span><strong>Local engines</strong><small>{engines.length ? "Detected on this machine." : "Open Harbor desktop to detect installed engines."}</small></span></div>
            </>
          ) : null}

          {page === "account" ? (
            <>
              <div className="harbor-settings-intro">
                <span className="harbor-eyebrow">THIS MACHINE</span>
                <h3>Account</h3>
                <p>Harbor is free and local. There is no cloud account, credit balance, or login behind this workspace.</p>
              </div>
              <div className="harbor-settings-list">
                <SettingRow label="Local profile" description="This name appears in the Harbor rail and local activity.">
                  <input aria-label="Local profile" value={accountName} maxLength={48} onChange={(event) => setAccountName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") saveProfile(); }} />
                </SettingRow>
              </div>
              <div className="harbor-settings-actions">
                <Button variant="primary" onClick={saveProfile}>Save profile</Button>
                <Button variant="ghost" onClick={() => { void settingsSet("onboarded_local", false); onShowWelcome(); }}>Show welcome again</Button>
              </div>
              <div className="harbor-settings-callout"><span className="harbor-chip">FREE · LOCAL</span><span><strong>Your files stay yours</strong><small>Workspace folders, conversations, and preferences remain on this computer.</small></span></div>
            </>
          ) : null}
        </section>
      </div>
    </div>
  );
}
