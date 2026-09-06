use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::Error,
    types::{PaneLayout, PaneState, RestoredPane, WorkspaceSetup, WorkspaceTab},
};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub async fn save(pool: &SqlitePool, tab_id: &str, layout: &PaneLayout) -> Result<(), Error> {
    let json = serde_json::to_string(layout)?;
    let result = sqlx::query("UPDATE workspace_tabs SET layout_json = ?1 WHERE id = ?2")
        .bind(json)
        .bind(tab_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("tab not found".into()));
    }
    Ok(())
}

pub async fn tidy(pool: &SqlitePool, tab_id: &str) -> Result<PaneLayout, Error> {
    let (json,): (String,) = sqlx::query_as("SELECT layout_json FROM workspace_tabs WHERE id = ?1")
        .bind(tab_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("tab not found".into()))?;
    let layout: PaneLayout =
        serde_json::from_str(&json).map_err(|_| Error::Message("invalid layout".into()))?;
    let tidied = tidy_tree(layout);
    save(pool, tab_id, &tidied).await?;
    Ok(tidied)
}

pub fn default_split(term_id: &str, files_id: &str) -> PaneLayout {
    PaneLayout::Split {
        dir: "h".into(),
        ratio: 0.5,
        a: Box::new(PaneLayout::Leaf {
            pane_id: term_id.into(),
        }),
        b: Box::new(PaneLayout::Leaf {
            pane_id: files_id.into(),
        }),
    }
}

async fn workspace_folder(pool: &SqlitePool, workspace_id: &str) -> Result<String, Error> {
    let folder: Option<(String,)> = sqlx::query_as("SELECT folder FROM workspaces WHERE id = ?1")
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?;
    folder
        .map(|(folder,)| folder)
        .ok_or_else(|| Error::Message("workspace not found".into()))
}

async fn load_tab(pool: &SqlitePool, tab_id: &str) -> Result<WorkspaceTab, Error> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id, workspace_id, layout_json FROM workspace_tabs WHERE id = ?1")
            .bind(tab_id)
            .fetch_optional(pool)
            .await?;
    let (id, workspace_id, json) = row.ok_or_else(|| Error::Message("tab not found".into()))?;
    let layout: PaneLayout =
        serde_json::from_str(&json).map_err(|_| Error::Message("invalid layout".into()))?;
    let panes = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT id, kind, paused, state_json FROM panes WHERE tab_id = ?1 ORDER BY created_at",
    )
    .bind(tab_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id, kind, paused, state_json)| RestoredPane {
        id,
        kind,
        paused: paused != 0,
        engine_id: serde_json::from_str::<serde_json::Value>(&state_json)
            .ok()
            .and_then(|state| {
                state
                    .get("engineId")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            }),
    })
    .collect();
    Ok(WorkspaceTab {
        id,
        workspace_id,
        layout,
        panes,
    })
}

async fn seed_default_panes(
    pool: &SqlitePool,
    tab_id: &str,
    folder: &str,
) -> Result<WorkspaceTab, Error> {
    let term_id = pane_create(
        pool,
        tab_id,
        "terminal",
        &PaneState {
            kind: "terminal".into(),
            cwd: Some(folder.into()),
            paused: Some(false),
            engine_id: None,
        },
    )
    .await?;
    let files_id = pane_create(
        pool,
        tab_id,
        "files",
        &PaneState {
            kind: "files".into(),
            cwd: Some(folder.into()),
            paused: Some(false),
            engine_id: None,
        },
    )
    .await?;
    let layout = default_split(&term_id, &files_id);
    save(pool, tab_id, &layout).await?;
    load_tab(pool, tab_id).await
}

/// Create a Code-mode tab with terminal + files leaves when the workspace has none.
pub async fn ensure_default_tab(
    pool: &SqlitePool,
    workspace_id: &str,
) -> Result<WorkspaceTab, Error> {
    let folder = workspace_folder(pool, workspace_id).await?;
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM workspace_tabs WHERE workspace_id = ?1 ORDER BY tab_order, created_at LIMIT 1",
    )
    .bind(workspace_id)
    .fetch_optional(pool)
    .await?;
    if let Some((tab_id,)) = existing {
        let tab = load_tab(pool, &tab_id).await?;
        if tab.panes.is_empty() {
            return seed_default_panes(pool, &tab_id, &folder).await;
        }
        return Ok(tab);
    }
    let tab_id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO workspace_tabs (id, workspace_id, title, tab_order, layout_json, created_at)
         VALUES (?1, ?2, 'Code', 0, ?3, ?4)",
    )
    .bind(&tab_id)
    .bind(workspace_id)
    .bind(serde_json::to_string(&default_split("term", "files"))?)
    .bind(now())
    .execute(pool)
    .await?;
    seed_default_panes(pool, &tab_id, &folder).await
}

fn is_default_tab(tab: &WorkspaceTab) -> bool {
    tab.panes.len() == 2
        && tab
            .panes
            .iter()
            .filter(|pane| pane.kind == "terminal")
            .count()
            == 1
        && tab.panes.iter().filter(|pane| pane.kind == "files").count() == 1
}

fn vertical_stack(pane_ids: &[String]) -> PaneLayout {
    debug_assert!(!pane_ids.is_empty());
    if pane_ids.len() == 1 {
        return PaneLayout::Leaf {
            pane_id: pane_ids[0].clone(),
        };
    }
    let ratio = if pane_ids.len() == 2 {
        0.5
    } else {
        1.0 / pane_ids.len() as f64
    };
    PaneLayout::Split {
        dir: "v".into(),
        ratio,
        a: Box::new(PaneLayout::Leaf {
            pane_id: pane_ids[0].clone(),
        }),
        b: Box::new(vertical_stack(&pane_ids[1..])),
    }
}

fn configured_layout(
    terminal_ids: &[String],
    browser_id: Option<&String>,
    thread_id: Option<&String>,
) -> PaneLayout {
    let primary = terminal_ids
        .first()
        .expect("a default workspace always has a terminal");

    // Keep the common two-terminal start exactly aligned with the reference
    // Code Hub layout. Extra terminals continue down the right-hand stack.
    if terminal_ids.len() == 2
        && let (Some(browser_id), Some(thread_id)) = (browser_id, thread_id)
    {
        return PaneLayout::Split {
            dir: "h".into(),
            ratio: 0.53,
            a: Box::new(PaneLayout::Leaf {
                pane_id: primary.clone(),
            }),
            b: Box::new(PaneLayout::Split {
                dir: "v".into(),
                ratio: 0.27,
                a: Box::new(PaneLayout::Leaf {
                    pane_id: terminal_ids[1].clone(),
                }),
                b: Box::new(PaneLayout::Split {
                    dir: "v".into(),
                    ratio: 0.5,
                    a: Box::new(PaneLayout::Leaf {
                        pane_id: browser_id.clone(),
                    }),
                    b: Box::new(PaneLayout::Leaf {
                        pane_id: thread_id.clone(),
                    }),
                }),
            }),
        };
    }

    let mut right_hand_panes = terminal_ids.iter().skip(1).cloned().collect::<Vec<_>>();
    if let Some(id) = browser_id {
        right_hand_panes.push(id.clone());
    }
    if let Some(id) = thread_id {
        right_hand_panes.push(id.clone());
    }
    if right_hand_panes.is_empty() {
        return PaneLayout::Leaf {
            pane_id: primary.clone(),
        };
    }

    PaneLayout::Split {
        dir: "h".into(),
        ratio: 0.53,
        a: Box::new(PaneLayout::Leaf {
            pane_id: primary.clone(),
        }),
        b: Box::new(vertical_stack(&right_hand_panes)),
    }
}

fn selected_terminal_engine(setup: &WorkspaceSetup, index: usize) -> String {
    let selected = setup
        .terminal_engine_ids
        .get(index)
        .map(String::as_str)
        .unwrap_or("shell")
        .trim();
    if selected == "shell"
        || crate::engines::catalog()
            .iter()
            .any(|spec| spec.id == selected && spec.supports_terminal)
    {
        selected.to_owned()
    } else {
        "shell".into()
    }
}

async fn set_pane_engine(pool: &SqlitePool, pane_id: &str, engine_id: &str) -> Result<(), Error> {
    let (state_json,): (String,) = sqlx::query_as("SELECT state_json FROM panes WHERE id = ?1")
        .bind(pane_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("pane not found".into()))?;
    let mut state: PaneState = serde_json::from_str(&state_json)
        .map_err(|_| Error::Message("invalid pane state".into()))?;
    state.engine_id = Some(engine_id.to_owned());
    sqlx::query("UPDATE panes SET state_json = ?1 WHERE id = ?2")
        .bind(serde_json::to_string(&state)?)
        .bind(pane_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Change the launch target of an existing terminal pane. The host performs
/// the executable check again when the PTY is restarted; the database only
/// stores a catalog id, never a renderer-provided path.
pub async fn set_engine(pool: &SqlitePool, pane_id: &str, engine_id: &str) -> Result<(), Error> {
    let (kind,): (String,) = sqlx::query_as("SELECT kind FROM panes WHERE id = ?1")
        .bind(pane_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::Message("pane not found".into()))?;
    if kind != "terminal" {
        return Err(Error::Message(
            "only terminal panes have a CLI engine".into(),
        ));
    }
    let selected = if engine_id.trim().is_empty() {
        "shell"
    } else {
        engine_id.trim()
    };
    if selected != "shell"
        && !crate::engines::catalog()
            .iter()
            .any(|spec| spec.id == selected && spec.supports_terminal)
    {
        return Err(Error::Message("unsupported terminal CLI".into()));
    }
    set_pane_engine(pool, pane_id, selected).await
}

/// Apply the one-time launch choices from the new-workspace dialog.
///
/// Existing, customized tabs are intentionally left alone. This makes
/// reopening an already configured folder safe while still letting a default
/// terminal + files tab become a useful Code Hub in one step.
pub async fn configure_workspace_tab(
    pool: &SqlitePool,
    workspace_id: &str,
    setup: &WorkspaceSetup,
) -> Result<WorkspaceTab, Error> {
    let folder = workspace_folder(pool, workspace_id).await?;
    let tab = ensure_default_tab(pool, workspace_id).await?;
    if !is_default_tab(&tab) {
        return Ok(tab);
    }

    let primary = tab
        .panes
        .iter()
        .find(|pane| pane.kind == "terminal")
        .expect("default workspace has a terminal");
    let legacy_files = tab.panes.iter().find(|pane| pane.kind == "files");
    let mut created = Vec::new();
    let extra_terminals = usize::from(setup.additional_terminals.min(4));

    let launch = async {
        let mut terminal_ids = vec![primary.id.clone()];
        for _ in 0..extra_terminals {
            let index = terminal_ids.len();
            let id = pane_create(
                pool,
                &tab.id,
                "terminal",
                &PaneState {
                    kind: "terminal".into(),
                    cwd: Some(folder.clone()),
                    paused: Some(false),
                    engine_id: Some(selected_terminal_engine(setup, index)),
                },
            )
            .await?;
            created.push(id.clone());
            terminal_ids.push(id);
        }

        let browser_id = if setup.browser_preview {
            let id = pane_create(
                pool,
                &tab.id,
                "browser",
                &PaneState {
                    kind: "browser".into(),
                    cwd: Some(folder.clone()),
                    paused: Some(false),
                    engine_id: None,
                },
            )
            .await?;
            created.push(id.clone());
            Some(id)
        } else {
            None
        };
        let thread_id = if setup.thread_pane {
            let id = pane_create(
                pool,
                &tab.id,
                "thread",
                &PaneState {
                    kind: "thread".into(),
                    cwd: Some(folder.clone()),
                    paused: Some(false),
                    engine_id: None,
                },
            )
            .await?;
            created.push(id.clone());
            Some(id)
        } else {
            None
        };

        Ok::<_, Error>((terminal_ids, browser_id, thread_id))
    }
    .await;
    let (terminal_ids, browser_id, thread_id) = match launch {
        Ok(value) => value,
        Err(error) => {
            for id in &created {
                let _ = pane_close(pool, id).await;
            }
            return Err(error);
        }
    };

    let next_layout = configured_layout(&terminal_ids, browser_id.as_ref(), thread_id.as_ref());
    if let Err(error) = save(pool, &tab.id, &next_layout).await {
        for id in &created {
            let _ = pane_close(pool, id).await;
        }
        return Err(error);
    }

    // The launch preset replaces the placeholder Files pane. It remains
    // available through the Code Hub's + menu when the user wants it.
    if let Some(files) = legacy_files {
        let _ = pane_close(pool, &files.id).await;
    }

    set_pane_engine(pool, &primary.id, &selected_terminal_engine(setup, 0)).await?;

    load_tab(pool, &tab.id).await
}

/// Load persisted tabs/panes. Does not spawn processes or restore scrollback.
/// Restored terminal leaves are marked paused; newly created tabs stay runnable.
pub async fn restore(pool: &SqlitePool) -> Result<Vec<WorkspaceTab>, Error> {
    let workspaces: Vec<(String,)> = sqlx::query_as("SELECT id FROM workspaces")
        .fetch_all(pool)
        .await?;
    let mut tabs = Vec::new();
    for (workspace_id,) in workspaces {
        let existed: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM workspace_tabs WHERE workspace_id = ?1 ORDER BY tab_order, created_at LIMIT 1",
        )
        .bind(&workspace_id)
        .fetch_optional(pool)
        .await?;
        let tab = ensure_default_tab(pool, &workspace_id).await?;
        if existed.is_some() {
            sqlx::query("UPDATE panes SET paused = 1 WHERE tab_id = ?1 AND kind = 'terminal'")
                .bind(&tab.id)
                .execute(pool)
                .await?;
            tabs.push(load_tab(pool, &tab.id).await?);
        } else {
            tabs.push(tab);
        }
    }
    Ok(tabs)
}

pub async fn pane_create(
    pool: &SqlitePool,
    tab_id: &str,
    kind: &str,
    state: &PaneState,
) -> Result<String, Error> {
    if kind.trim().is_empty() {
        return Err(Error::Message("kind required".into()));
    }
    let tab: Option<(String,)> = sqlx::query_as("SELECT id FROM workspace_tabs WHERE id = ?1")
        .bind(tab_id)
        .fetch_optional(pool)
        .await?;
    if tab.is_none() {
        return Err(Error::Message("tab not found".into()));
    }
    let id = Uuid::now_v7().to_string();
    let paused = i64::from(state.paused.unwrap_or(false));
    sqlx::query(
        "INSERT INTO panes (id, tab_id, kind, state_json, paused, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&id)
    .bind(tab_id)
    .bind(kind)
    .bind(serde_json::to_string(state)?)
    .bind(paused)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn pane_close(pool: &SqlitePool, id: &str) -> Result<(), Error> {
    let result = sqlx::query("DELETE FROM panes WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::Message("pane not found".into()));
    }
    Ok(())
}

fn tidy_tree(layout: PaneLayout) -> PaneLayout {
    match layout {
        PaneLayout::Leaf { pane_id } => PaneLayout::Leaf { pane_id },
        PaneLayout::Tabs { active, kids } => PaneLayout::Tabs { active, kids },
        PaneLayout::Split { dir, a, b, .. } => {
            let a = Box::new(tidy_tree(*a));
            let b = Box::new(tidy_tree(*b));
            let left = leaf_count(&a);
            let right = leaf_count(&b);
            let total = (left + right).max(1);
            let ratio = left as f64 / total as f64;
            PaneLayout::Split { dir, ratio, a, b }
        }
    }
}

fn leaf_count(layout: &PaneLayout) -> usize {
    match layout {
        PaneLayout::Leaf { .. } => 1,
        PaneLayout::Split { a, b, .. } => leaf_count(a) + leaf_count(b),
        PaneLayout::Tabs { kids, .. } => kids.len().max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn tab(
        pool: &SqlitePool,
        workspace_folder: &std::path::Path,
        layout: &PaneLayout,
    ) -> String {
        let workspace = crate::workspaces::add(pool, workspace_folder.display().to_string())
            .await
            .unwrap();
        let ensured = ensure_default_tab(pool, &workspace.id).await.unwrap();
        save(pool, &ensured.id, layout).await.unwrap();
        ensured.id
    }

    #[tokio::test]
    async fn save_tidy_panes_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let folder = dir.path().join("ws");
        std::fs::create_dir(&folder).unwrap();
        let layout = PaneLayout::Split {
            dir: "h".into(),
            ratio: 0.8,
            a: Box::new(PaneLayout::Leaf {
                pane_id: "a".into(),
            }),
            b: Box::new(PaneLayout::Split {
                dir: "h".into(),
                ratio: 0.9,
                a: Box::new(PaneLayout::Leaf {
                    pane_id: "b".into(),
                }),
                b: Box::new(PaneLayout::Leaf {
                    pane_id: "c".into(),
                }),
            }),
        };
        let tab_id = tab(&pool, &folder, &layout).await;
        save(&pool, &tab_id, &layout).await.unwrap();
        let tidied = tidy(&pool, &tab_id).await.unwrap();
        match &tidied {
            PaneLayout::Split { ratio, b, .. } => {
                assert!((ratio - (1.0 / 3.0)).abs() < 1e-9);
                match b.as_ref() {
                    PaneLayout::Split { ratio, .. } => {
                        assert!((ratio - 0.5).abs() < 1e-9);
                    }
                    other => panic!("expected inner split, got {other:?}"),
                }
            }
            other => panic!("expected split, got {other:?}"),
        }
        assert!(save(&pool, "missing", &layout).await.is_err());

        let pane_id = pane_create(
            &pool,
            &tab_id,
            "terminal",
            &PaneState {
                kind: "terminal".into(),
                cwd: Some("/tmp".into()),
                paused: Some(false),
                engine_id: None,
            },
        )
        .await
        .unwrap();
        let (paused,): (i64,) = sqlx::query_as("SELECT paused FROM panes WHERE id = ?1")
            .bind(&pane_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(paused, 0);
        restore(&pool).await.unwrap();
        let (paused,): (i64,) = sqlx::query_as("SELECT paused FROM panes WHERE id = ?1")
            .bind(&pane_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(paused, 1);
        pane_close(&pool, &pane_id).await.unwrap();
        assert!(pane_close(&pool, &pane_id).await.is_err());
        restore(&pool).await.unwrap();
    }

    #[test]
    fn pane_layout_uses_camel_case_over_ipc_and_reads_legacy_json() {
        let layout = default_split("terminal", "files");
        let value = serde_json::to_value(&layout).unwrap();
        assert_eq!(value["a"]["paneId"], "terminal");
        assert!(value["a"].get("pane_id").is_none());

        let legacy = r#"{
          "type":"split",
          "dir":"h",
          "ratio":0.5,
          "a":{"type":"leaf","pane_id":"terminal"},
          "b":{"type":"leaf","pane_id":"files"}
        }"#;
        let parsed: PaneLayout = serde_json::from_str(legacy).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap()["b"]["paneId"],
            "files"
        );
    }

    #[tokio::test]
    async fn opening_a_folder_creates_a_tab_that_restore_can_load() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let folder = dir.path().join("ws");
        std::fs::create_dir(&folder).unwrap();
        let workspace = crate::workspaces::add(&pool, folder.display().to_string())
            .await
            .unwrap();
        let first = ensure_default_tab(&pool, &workspace.id).await.unwrap();
        let again = ensure_default_tab(&pool, &workspace.id).await.unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(first.workspace_id, workspace.id);
        assert!(first.panes.iter().any(|pane| pane.kind == "terminal"));
        assert!(first.panes.iter().any(|pane| pane.kind == "files"));
        save(&pool, &first.id, &first.layout).await.unwrap();

        let restored = restore(&pool).await.unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, first.id);
        assert!(
            restored[0]
                .panes
                .iter()
                .any(|pane| pane.kind == "terminal" && pane.paused)
        );
        match &restored[0].layout {
            PaneLayout::Split { a, b, .. } => {
                assert!(matches!(a.as_ref(), PaneLayout::Leaf { .. }));
                assert!(matches!(b.as_ref(), PaneLayout::Leaf { .. }));
            }
            other => panic!("expected split, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn workspace_setup_creates_the_requested_code_hub_start() {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let folder = dir.path().join("project");
        std::fs::create_dir(&folder).unwrap();
        let workspace = crate::workspaces::add(&pool, folder.display().to_string())
            .await
            .unwrap();

        let configured = configure_workspace_tab(
            &pool,
            &workspace.id,
            &WorkspaceSetup {
                additional_terminals: 1,
                browser_preview: true,
                thread_pane: true,
                terminal_engine_ids: vec!["claude-code".into(), "shell".into()],
            },
        )
        .await
        .unwrap();

        assert_eq!(
            configured
                .panes
                .iter()
                .filter(|pane| pane.kind == "terminal")
                .count(),
            2
        );
        assert!(configured.panes.iter().any(|pane| pane.kind == "browser"));
        assert!(configured.panes.iter().any(|pane| pane.kind == "thread"));
        assert!(!configured.panes.iter().any(|pane| pane.kind == "files"));
        let terminals = configured
            .panes
            .iter()
            .filter(|pane| pane.kind == "terminal")
            .collect::<Vec<_>>();
        assert_eq!(terminals[0].engine_id.as_deref(), Some("claude-code"));
        assert_eq!(terminals[1].engine_id.as_deref(), Some("shell"));
        set_engine(&pool, &terminals[0].id, "opencode")
            .await
            .unwrap();
        let switched = load_tab(&pool, &configured.id).await.unwrap();
        assert_eq!(
            switched
                .panes
                .iter()
                .find(|pane| pane.id == terminals[0].id)
                .and_then(|pane| pane.engine_id.as_deref()),
            Some("opencode")
        );
        assert!(
            set_engine(
                &pool,
                &configured
                    .panes
                    .iter()
                    .find(|pane| pane.kind == "browser")
                    .unwrap()
                    .id,
                "shell"
            )
            .await
            .is_err()
        );
        match configured.layout {
            PaneLayout::Split { ratio, .. } => assert!((ratio - 0.53).abs() < 1e-9),
            other => panic!("expected a split Code Hub layout, got {other:?}"),
        }

        // A second launch choice must not overwrite a customized tab.
        let reopened = configure_workspace_tab(
            &pool,
            &workspace.id,
            &WorkspaceSetup {
                additional_terminals: 4,
                browser_preview: false,
                thread_pane: false,
                terminal_engine_ids: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(reopened.panes.len(), configured.panes.len());

        let minimal_folder = dir.path().join("minimal");
        std::fs::create_dir(&minimal_folder).unwrap();
        let minimal_workspace = crate::workspaces::add(&pool, minimal_folder.display().to_string())
            .await
            .unwrap();
        let minimal = configure_workspace_tab(
            &pool,
            &minimal_workspace.id,
            &WorkspaceSetup {
                additional_terminals: 0,
                browser_preview: false,
                thread_pane: false,
                terminal_engine_ids: vec!["shell".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(minimal.panes.len(), 1);
        assert!(matches!(minimal.layout, PaneLayout::Leaf { .. }));
    }
}
