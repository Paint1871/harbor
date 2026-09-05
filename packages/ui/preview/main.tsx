import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider, useTheme } from "../src/ThemeProvider";
import { forceDark } from "../src/theme";
import type { Theme } from "../src/theme";
import { Button } from "../src/Button";
import { Segmented } from "../src/Segmented";
import { RailRow } from "../src/RailRow";
import { Composer } from "../src/Composer";
import { Card } from "../src/Card";
import { Pill } from "../src/Pill";
import "../src/tokens.css";
import "../src/primitives.css";
import "./preview.css";

forceDark();

const modes = [
  { value: "agent", label: "Agent" },
  { value: "code", label: "Code" },
  { value: "chat", label: "Chat" },
] as const;
const themes = [{ value: "black", label: "Black" }, { value: "light", label: "Light" }] as const;
const swatches = ["bg", "surface", "raised", "accent-speak", "accent-think", "accent-error"];

function MotionSample() {
  const { reducedMotion } = useTheme();
  return (
    <div className="motion-sample">
      <span className="motion-dot" aria-hidden="true" />
      <span data-motion-state>{reducedMotion ? "Stillframe" : "Motion enabled"}</span>
    </div>
  );
}

function Preview() {
  const [theme, setTheme] = useState<Theme>("black");
  const [reduceMotion, setReduceMotion] = useState(false);
  const [disabled, setDisabled] = useState(false);
  const [mode, setMode] = useState("agent");
  const [selected, setSelected] = useState("release");
  const [draft, setDraft] = useState("");
  const [sent, setSent] = useState<string[]>([]);
  const [action, setAction] = useState("Ready");

  return (
    <ThemeProvider theme={theme} reduceMotion={reduceMotion}>
      <main className="preview">
        <header className="preview-header">
          <div>
            <p className="eyebrow">HARBOR / UI LIBRARY</p>
            <h1>A quiet place to work.</h1>
            <p className="intro">Design tokens and primitives · development preview</p>
          </div>
          <div className="preview-preferences">
            <Segmented label="Appearance" value={theme} options={themes} onValueChange={setTheme} />
            <label><input type="checkbox" checked={reduceMotion} onChange={(event) => setReduceMotion(event.target.checked)} /> Reduce Motion</label>
          </div>
        </header>

        <section className="swatches" aria-label="Surface and accent tokens">
          {swatches.map((name) => (
            <div className="swatch" key={name}>
              <div style={{ background: `var(--harbor-${name})` }} />
              <span>{name.replace("accent-", "")}</span>
            </div>
          ))}
        </section>

        <div className="preview-grid">
          <section aria-labelledby="rail-heading">
            <h2 id="rail-heading">01 / Rail rows</h2>
            <Card className="rail-sample">
              <RailRow label="Release manager" description="The checks are ready to review." leading={<span className="initials">RM</span>} trailing={<Pill tone="live">2</Pill>} selected={selected === "release"} onClick={() => setSelected("release")} />
              <RailRow label="Design partner" description="A note on spacing and contrast." leading={<span className="initials violet">DP</span>} trailing="2h" selected={selected === "design"} onClick={() => setSelected("design")} />
              <RailRow label="Research partner" description="Waiting for an engine." leading={<span className="initials amber">RP</span>} disabled />
            </Card>
            <h2>02 / Pills</h2>
            <div className="sample-row">
              <Pill>Free · local</Pill>
              <Pill tone="attention">Needs you</Pill>
            </div>
            <h2>03 / Motion</h2>
            <Card><MotionSample /></Card>
          </section>

          <section aria-labelledby="controls-heading">
            <h2 id="controls-heading">04 / Controls</h2>
            <Card>
              <div className="controls-row">
                <Segmented label="Mode sample" value={mode} options={modes} onValueChange={setMode} disabled={disabled} />
                <label className="muted"><input type="checkbox" checked={disabled} onChange={(event) => setDisabled(event.target.checked)} /> Disable controls</label>
              </div>
              <div className="sample-row button-samples">
                <Button variant="primary" disabled={disabled} onClick={() => setAction("Primary action selected")}>Start local</Button>
                <Button disabled={disabled} onClick={() => setAction("Secondary action selected")}>Local profile</Button>
                <Button variant="ghost" disabled={disabled} onClick={() => setAction("Ghost action selected")}>Cancel</Button>
                <Button size="icon" aria-label="Add item" disabled={disabled} onClick={() => setAction("Icon action selected")}>+</Button>
              </div>
              <p className="sample-feedback" role="status">{action} · Selected mode: {mode}</p>
            </Card>
            <h2>05 / Composer</h2>
            <Composer value={draft} onValueChange={setDraft} disabled={disabled} onSend={(value) => { setSent((messages) => [...messages, value]); setDraft(""); }} controls={<span className="muted">Enter to send · Shift+Enter for a new line</span>} />
            <div className="composer-result" role="status">
              <span className="eyebrow">LOCAL PREVIEW · {sent.length} SENT</span>
              <p>{sent.at(-1) ?? "Your message stays in this preview. No engine is connected."}</p>
            </div>
            <h2>06 / Dimensions</h2>
            <div className="dimensions">
              <div><span className="orb-size" />Orb seat · 28 px</div>
              <div><span className="tab-size" />Tab · 32 px</div>
              <div><span className="spacing-sample"><i /><i /><i /><i /></span>Space · 4 / 8 / 12 / 16</div>
            </div>
          </section>
        </div>
      </main>
    </ThemeProvider>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("UI preview root is missing");
createRoot(root).render(<StrictMode><Preview /></StrictMode>);
