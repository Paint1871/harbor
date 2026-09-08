import { useEffect, useMemo, useState } from "react";
import { Button } from "@harbor/ui/Button";
import { settingsSet } from "../settings";
import { useChrome } from "../chrome/chrome-context";
import { SKILLS } from "../skills/catalog";

const DEFAULT_SKILL = SKILLS[0]!;

export function Skills() {
  const { onModeChange } = useChrome();
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState(DEFAULT_SKILL.id);
  const [notice, setNotice] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<"use" | "copy" | null>(null);
  const selected = SKILLS.find((skill) => skill.id === selectedId) ?? DEFAULT_SKILL;
  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return SKILLS;
    return SKILLS.filter((skill) => `${skill.name} ${skill.label} ${skill.description} ${skill.tags.join(" ")}`.toLowerCase().includes(needle));
  }, [query]);

  useEffect(() => {
    if (visible.length && !visible.some((skill) => skill.id === selectedId)) {
      setSelectedId(visible[0]!.id);
      setNotice(null);
    }
  }, [selectedId, visible]);

  async function useSkill() {
    if (busyAction) return;
    setBusyAction("use");
    setNotice(null);
    try {
      await settingsSet("pending_agent_prompt", selected.brief);
      setNotice(`${selected.name} is ready in the Agent composer.`);
      onModeChange("agent");
    } catch (reason) {
      setNotice(`This skill could not be opened. ${String(reason)}`);
    } finally {
      setBusyAction(null);
    }
  }

  async function copyBrief() {
    if (busyAction) return;
    setBusyAction("copy");
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(selected.brief);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = selected.brief;
        textarea.setAttribute("readonly", "true");
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        if (!document.execCommand("copy")) throw new Error("Clipboard access is unavailable in this window.");
        textarea.remove();
      }
      setNotice("Skill brief copied to the clipboard.");
    } catch (reason) {
      setNotice(String(reason));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="harbor-destination-page harbor-skills" aria-label="Skills">
      <header className="harbor-destination-heading">
        <div>
          <span className="harbor-eyebrow">LOCAL PLAYBOOKS</span>
          <h1>Skills</h1>
          <p>Reusable briefs for the work your teammates do most often. Pick one, tune the wording, and open it in Agent mode.</p>
        </div>
        <label className="harbor-destination-search">Search skills<input aria-label="Search skills" value={query} placeholder="Find a playbook" onChange={(event) => setQuery(event.target.value)} /></label>
      </header>

      <div className="harbor-skill-layout">
        <div className="harbor-skill-list" aria-label="Skill library">
          {visible.length ? visible.map((skill) => (
            <button key={skill.id} type="button" className="harbor-skill-row" aria-pressed={selected.id === skill.id} data-selected={selected.id === skill.id} onClick={() => { setSelectedId(skill.id); setNotice(null); }}>
              <span className="harbor-skill-row-mark" aria-hidden="true">✦</span>
              <span><strong>{skill.name}</strong><small>{skill.label}</small></span>
              <span className="harbor-skill-row-arrow" aria-hidden="true">→</span>
            </button>
          )) : <p className="harbor-muted">No skills match “{query}”.</p>}
        </div>
        <article className="harbor-skill-detail">
          <div className="harbor-skill-detail-top"><span className="harbor-skill-detail-mark" aria-hidden="true">✦</span><div><span className="harbor-eyebrow">SKILL BRIEF</span><h2>{selected.name}</h2></div></div>
          <p>{selected.description}</p>
          <div className="harbor-skill-tags">{selected.tags.map((tag) => <span key={tag} className="harbor-chip">{tag}</span>)}</div>
          <div className="harbor-skill-brief"><span className="harbor-eyebrow">STARTING BRIEF</span><p>{selected.brief}</p></div>
          <div className="harbor-skill-actions"><Button variant="primary" disabled={busyAction !== null || !visible.length} onClick={() => void useSkill()}>{busyAction === "use" ? "Opening…" : "Use in Agent"} <span aria-hidden="true">↗</span></Button><Button variant="ghost" disabled={busyAction !== null || !visible.length} onClick={() => void copyBrief()}>{busyAction === "copy" ? "Copying…" : "Copy brief"}</Button></div>
        </article>
      </div>
      {notice ? <p className="harbor-destination-notice" role="status">{notice}</p> : null}
    </section>
  );
}
