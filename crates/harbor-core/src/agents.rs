use std::collections::HashSet;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::Error,
    types::{AgentRecord, AgentTrailing, CreateAgent, UpdateAgent},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

type AgentRow = (
    String,
    String,
    String,
    String,
    i64,
    i64,
    String,
    i64,
    Option<String>,
    i64,
    i64,
    Option<i64>,
);

pub async fn list(pool: &SqlitePool) -> Result<Vec<AgentRecord>, Error> {
    let rows = sqlx::query_as::<_, AgentRow>(
        "SELECT a.id, a.name, a.brief, a.engine_id, a.face_index, a.pinned, a.home_path, a.messaging,
                (
                    SELECT m.prose FROM messages m
                    JOIN agent_chats c ON c.id = m.chat_id
                    WHERE c.agent_id = a.id
                      AND m.chat_kind = 'agent'
                      AND m.role IN ('user', 'assistant', 'mail')
                    ORDER BY m.created_at DESC, m.rowid DESC
                    LIMIT 1
                ),
                (SELECT COUNT(*) FROM agent_chats WHERE agent_id = a.id AND status = 'running'),
                (SELECT COUNT(*) FROM agent_chats WHERE agent_id = a.id AND status = 'needs_you'),
                (SELECT MAX(updated_at) FROM agent_chats WHERE agent_id = a.id)
         FROM agents a
         ORDER BY a.pinned DESC, a.pin_order, a.name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(map_row).collect())
}

fn map_row(
    (
        id,
        name,
        brief,
        engine_id,
        face_index,
        pinned,
        home_path,
        messaging,
        last_line,
        running,
        needs_you,
        last_spoke,
    ): AgentRow,
) -> AgentRecord {
    AgentRecord {
        id,
        name,
        brief,
        engine_id,
        face_index: face_index as i32,
        pinned: pinned != 0,
        home_path,
        messaging: messaging != 0,
        last_line: clip_last_line(last_line),
        trailing: trailing_from(running, needs_you, last_spoke),
    }
}

fn trailing_from(running: i64, needs_you: i64, last_spoke: Option<i64>) -> AgentTrailing {
    if running > 0 {
        AgentTrailing::Running {
            n: running.min(i32::MAX as i64) as i32,
        }
    } else if needs_you > 0 {
        AgentTrailing::NeedsYou
    } else if let Some(at) = last_spoke {
        AgentTrailing::LastSpoke { at }
    } else {
        AgentTrailing::Idle
    }
}

fn clip_last_line(prose: Option<String>) -> Option<String> {
    let collapsed = prose?.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let extra = collapsed.chars().count() > 80;
    let mut clipped: String = collapsed.chars().take(80).collect();
    if extra {
        clipped.push('…');
    }
    Some(clipped)
}

pub async fn create(pool: &SqlitePool, input: CreateAgent) -> Result<AgentRecord, Error> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(Error::Message("name required".into()));
    }
    let id = Uuid::now_v7().to_string();
    let ts = now();
    sqlx::query(
        "INSERT INTO agents (id, name, brief, engine_id, face_index, home_path, messaging, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&input.brief)
    .bind(&input.engine_id)
    .bind(input.face_index)
    .bind("")
    .bind(ts)
    .execute(pool)
    .await
    .map_err(|err| Error::from_constraint(err, "An agent with this name already exists."))?;
    Ok(AgentRecord {
        id,
        name,
        brief: input.brief,
        engine_id: input.engine_id,
        face_index: input.face_index,
        pinned: false,
        home_path: String::new(),
        messaging: true,
        last_line: None,
        trailing: AgentTrailing::Idle,
    })
}

pub async fn update(pool: &SqlitePool, input: UpdateAgent) -> Result<(), Error> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM agents WHERE id = ?1")
        .bind(&input.id)
        .fetch_optional(pool)
        .await?;
    if exists.is_none() {
        return Err(Error::Message("agent not found".into()));
    }

    let name = match input.name {
        Some(name) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(Error::Message("name required".into()));
            }
            Some(name)
        }
        None => None,
    };

    let ts = now();
    let pinned = input.pinned.map(i64::from);
    let messaging = input.messaging.map(i64::from);
    let home_path = match input.home_path {
        Some(path) if path.trim().is_empty() => Some(String::new()),
        Some(path) => Some(crate::places::canonicalize_folder(&path)?),
        None => None,
    };
    sqlx::query(
        "UPDATE agents SET
            name = COALESCE(?1, name),
            brief = COALESCE(?2, brief),
            engine_id = COALESCE(?3, engine_id),
            face_index = COALESCE(?4, face_index),
            pinned = COALESCE(?5, pinned),
            pin_order = CASE
                WHEN ?5 IS NULL THEN pin_order
                WHEN ?5 = 1 THEN COALESCE(pin_order, ?6)
                ELSE NULL
            END,
            home_path = COALESCE(?7, home_path),
            messaging = COALESCE(?8, messaging),
            updated_at = ?6
         WHERE id = ?9",
    )
    .bind(name)
    .bind(input.brief)
    .bind(input.engine_id)
    .bind(input.face_index)
    .bind(pinned)
    .bind(ts)
    .bind(home_path)
    .bind(messaging)
    .bind(&input.id)
    .execute(pool)
    .await
    .map_err(|err| Error::from_constraint(err, "An agent with this name already exists."))?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    sqlx::query(
        "DELETE FROM messages WHERE chat_kind = 'agent' AND chat_id IN (
            SELECT id FROM agent_chats WHERE agent_id = ?1
        )",
    )
    .bind(id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM agents WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn require(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    let found: Option<(String,)> = sqlx::query_as("SELECT id FROM agents WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        Err(Error::Message("agent not found".into()))
    } else {
        Ok(())
    }
}

const DEFAULT_ENGINE: &str = "opencode";
const NAME_MAX: usize = 40;

/// Local draft helper: derives a name and brief from `hint` without a
/// network call or vendor API key. This is not an LLM.
pub async fn draft_with_ai(pool: &SqlitePool, hint: String) -> Result<CreateAgent, Error> {
    let hint = hint.trim();
    let name_seed = name_from_hint(hint);
    let taken: HashSet<String> = sqlx::query_as::<_, (String,)>("SELECT name FROM agents")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|(name,)| name)
        .collect();
    let name = unique_name(&name_seed, &taken);
    let brief = if hint.is_empty() {
        "A local coding teammate.".into()
    } else {
        hint.to_string()
    };
    Ok(CreateAgent {
        name,
        brief,
        engine_id: DEFAULT_ENGINE.into(),
        face_index: 0,
    })
}

fn name_from_hint(hint: &str) -> String {
    let collapsed = hint.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return "New agent".into();
    }
    clip_chars(&collapsed, NAME_MAX)
}

fn unique_name(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let suffix = format!(" {n}");
        let max_base = NAME_MAX.saturating_sub(suffix.chars().count());
        let candidate = format!("{}{suffix}", clip_chars(base, max_base));
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    clip_chars(&format!("{base}-draft"), NAME_MAX)
}

fn clip_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

const PALETTES: [(u8, u8, u8); 12] = [
    (61, 90, 128),
    (238, 108, 77),
    (152, 193, 217),
    (41, 50, 65),
    (224, 122, 95),
    (129, 178, 154),
    (242, 204, 143),
    (61, 64, 91),
    (92, 77, 125),
    (42, 157, 143),
    (233, 196, 106),
    (38, 70, 83),
];

/// Stable SVG data URL from initials and palette index 0–11.
/// Does not fetch CDN faces. An on-disk atlas is optional and unused here.
pub async fn face_preview(
    pool: &SqlitePool,
    agent_id: &str,
    face_index: i32,
) -> Result<String, Error> {
    let name: String = sqlx::query_as::<_, (String,)>("SELECT name FROM agents WHERE id = ?1")
        .bind(agent_id)
        .fetch_optional(pool)
        .await?
        .map(|(name,)| name)
        .unwrap_or_else(|| agent_id.to_string());
    Ok(face_svg_data_url(&name, face_index))
}

fn face_svg_data_url(name: &str, face_index: i32) -> String {
    let palette = face_index.rem_euclid(12) as usize;
    let (r, g, b) = PALETTES[palette];
    let initials = initials(name);
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>\
<rect width='64' height='64' rx='32' fill='rgb({r},{g},{b})'/>\
<text x='32' y='38' text-anchor='middle' font-family='sans-serif' font-size='22' fill='rgb(245,245,245)'>{initials}</text>\
</svg>"
    );
    format!(
        "data:image/svg+xml;charset=utf-8,{}",
        svg.replace(' ', "%20")
    )
}

fn initials(name: &str) -> String {
    let letters: Vec<char> = name
        .split_whitespace()
        .filter_map(|part| part.chars().find(|ch| ch.is_alphabetic()))
        .take(2)
        .collect();
    let raw = if letters.len() >= 2 {
        format!("{}{}", letters[0], letters[1])
    } else if letters.len() == 1 {
        let second = name
            .chars()
            .filter(|ch| ch.is_alphabetic())
            .nth(1)
            .unwrap_or(letters[0]);
        format!("{}{second}", letters[0])
    } else {
        let alnum: String = name
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .take(2)
            .collect();
        if alnum.is_empty() { "??".into() } else { alnum }
    };
    raw.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn pool() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        (dir, pool)
    }

    fn sample(name: &str) -> CreateAgent {
        CreateAgent {
            name: name.into(),
            brief: "brief".into(),
            engine_id: "opencode".into(),
            face_index: 3,
        }
    }

    #[tokio::test]
    async fn update_applies_all_fields_and_rejects_bad_names() {
        let (_dir, pool) = pool().await;
        let agent = create(&pool, sample("Alpha")).await.unwrap();
        update(
            &pool,
            UpdateAgent {
                id: agent.id.clone(),
                name: Some("Beta".into()),
                brief: Some("new brief".into()),
                engine_id: Some("claude".into()),
                face_index: Some(4),
                pinned: Some(true),
                home_path: None,
                messaging: Some(false),
            },
        )
        .await
        .unwrap();
        let listed = list(&pool).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Beta");
        assert_eq!(listed[0].brief, "new brief");
        assert_eq!(listed[0].engine_id, "claude");
        assert_eq!(listed[0].face_index, 4);
        assert!(listed[0].pinned);
        assert!(!listed[0].messaging);
        assert_eq!(listed[0].trailing, AgentTrailing::Idle);

        let err = update(
            &pool,
            UpdateAgent {
                id: agent.id.clone(),
                name: Some("   ".into()),
                brief: None,
                engine_id: None,
                face_index: None,
                pinned: None,
                home_path: None,
                messaging: None,
            },
        )
        .await
        .unwrap_err();
        assert_eq!(err.to_string(), "name required");

        create(&pool, sample("Gamma")).await.unwrap();
        let duplicate = update(
            &pool,
            UpdateAgent {
                id: agent.id,
                name: Some("Gamma".into()),
                brief: None,
                engine_id: None,
                face_index: None,
                pinned: None,
                home_path: None,
                messaging: None,
            },
        )
        .await
        .unwrap_err();
        assert_eq!(
            duplicate.to_string(),
            "An agent with this name already exists."
        );
    }

    #[tokio::test]
    async fn create_maps_unique_name_and_draft_is_local() {
        let (_dir, pool) = pool().await;
        create(&pool, sample("Solo")).await.unwrap();
        let err = create(&pool, sample("Solo")).await.unwrap_err();
        assert_eq!(err.to_string(), "An agent with this name already exists.");

        let first = draft_with_ai(&pool, "release manager for payments".into())
            .await
            .unwrap();
        assert_eq!(first.engine_id, "opencode");
        assert_eq!(first.face_index, 0);
        assert!(first.name.chars().count() <= 40 && !first.name.is_empty());
        assert!(first.brief.contains("payments"));
        create(&pool, first.clone()).await.unwrap();
        let second = draft_with_ai(&pool, "release manager for payments".into())
            .await
            .unwrap();
        assert_ne!(first.name, second.name);
        assert!(second.name.chars().count() <= 40);

        let empty = draft_with_ai(&pool, "   ".into()).await.unwrap();
        assert_eq!(empty.name, "New agent");
        assert!(!empty.brief.is_empty());
    }

    #[tokio::test]
    async fn face_preview_is_a_stable_svg_data_url() {
        let (_dir, pool) = pool().await;
        let agent = create(&pool, sample("Ada Lovelace")).await.unwrap();
        let url = face_preview(&pool, &agent.id, 0).await.unwrap();
        assert!(url.starts_with("data:image/svg+xml"));
        assert!(url.contains("AL") || url.contains("Ada") || url.contains("svg"));
        let again = face_preview(&pool, &agent.id, 0).await.unwrap();
        assert_eq!(url, again);
        let other = face_preview(&pool, &agent.id, 1).await.unwrap();
        assert_ne!(url, other);
    }

    #[tokio::test]
    async fn list_includes_last_line_trailing_and_home() {
        let (_dir, pool) = pool().await;
        let agent = create(&pool, sample("Ada Lovelace")).await.unwrap();
        assert!(agent.messaging);
        let home = _dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        update(
            &pool,
            UpdateAgent {
                id: agent.id.clone(),
                name: None,
                brief: None,
                engine_id: None,
                face_index: None,
                pinned: None,
                home_path: Some(home.display().to_string()),
                messaging: None,
            },
        )
        .await
        .unwrap();

        let chat = crate::chats::create(&pool, &agent.id).await.unwrap();
        crate::chats::send(
            &pool,
            &chat.id,
            &[crate::types::ContentPart {
                r#type: "text".into(),
                text: Some("Ship the checklist today".into()),
                path: None,
            }],
        )
        .await
        .unwrap();
        crate::chats::set_status(&pool, &chat.id, "running")
            .await
            .unwrap();

        let listed = list(&pool).await.unwrap();
        assert_eq!(
            listed[0].last_line.as_deref(),
            Some("Ship the checklist today")
        );
        assert_eq!(listed[0].trailing, AgentTrailing::Running { n: 1 });
        assert!(listed[0].home_path.ends_with("home"));

        crate::chats::set_status(&pool, &chat.id, "needs_you")
            .await
            .unwrap();
        let listed = list(&pool).await.unwrap();
        assert_eq!(listed[0].trailing, AgentTrailing::NeedsYou);
    }
}
