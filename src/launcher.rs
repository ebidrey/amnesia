use chrono::Utc;
use dialoguer::{Input, Select};
use ulid::Ulid;

use crate::model::Session;
use crate::{config, projects, sessions};

const KNOWN_ORCHESTRATORS: &[&str] = &["claude", "opencode", "cursor", "aider", "goose"];

/// Returns only the names from `candidates` for which `which <name>` succeeds.
fn filter_installed<'a>(candidates: &[&'a str]) -> Vec<&'a str> {
    candidates
        .iter()
        .filter(|&&name| {
            std::process::Command::new("which")
                .arg(name)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
        .copied()
        .collect()
}

fn detect_installed_orchestrators() -> Vec<&'static str> {
    filter_installed(KNOWN_ORCHESTRATORS)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load projects
    let mut config = projects::load();

    // 2. Build selection list: project names + "+ New project"
    let new_project_item = "+ New project";
    let mut items: Vec<String> = config
        .projects
        .iter()
        .map(|p| p.name.clone())
        .collect();
    items.push(new_project_item.to_string());

    // 3. Show project selector
    let project_selection = Select::new()
        .with_prompt("Select a project:")
        .items(&items)
        .default(0)
        .interact()?;

    let project_name = if items[project_selection] == new_project_item {
        // 4. Ask for new project name, add and save
        let name: String = Input::new()
            .with_prompt("Project name")
            .interact_text()?;
        config.projects.push(projects::Project { name: name.clone() });
        projects::save(&config)?;
        name
    } else {
        items[project_selection].clone()
    };

    // 5. Detect and select orchestrator
    let orchestrators = detect_installed_orchestrators();

    if orchestrators.is_empty() {
        return Err(
            "no supported orchestrator found — install claude, opencode, cursor or aider".into(),
        );
    }

    let orch_selection = Select::new()
        .with_prompt("Orchestrator:")
        .items(&orchestrators)
        .default(0)
        .interact()?;

    let orchestrator = orchestrators[orch_selection];

    // 6. Session selection
    let sessions_path = config::project_sessions_path(&project_name);
    let mut existing = sessions::load_from(&sessions_path).unwrap_or_default();
    existing.sort_by(|a, b| b.id.cmp(&a.id));
    existing.truncate(5);

    let new_session_item = "+ New session".to_string();
    let session_items: Vec<String> = std::iter::once(new_session_item.clone())
        .chain(existing.iter().map(|s| {
            let date = &s.started_at[..16].replace('T', " ");
            format!("{}  {}  {}", date, s.orchestrator, &s.id[..8])
        }))
        .collect();

    let session_selection = Select::new()
        .with_prompt("Session:")
        .items(&session_items)
        .default(0)
        .interact()?;

    let session_id = if session_items[session_selection] == new_session_item {
        let id = Ulid::new().to_string();
        let session = Session {
            id: id.clone(),
            project: project_name.clone(),
            orchestrator: orchestrator.to_string(),
            started_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        };
        sessions::append_to(&sessions_path, &session)?;
        id
    } else {
        existing[session_selection - 1].id.clone()
    };

    // 7. Launch
    eprintln!("-> Launching {orchestrator}...");

    let status = std::process::Command::new(orchestrator)
        .env("AMNESIA_PROJECT", &project_name)
        .env("AMNESIA_SESSION", &session_id)
        .status()?;

    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- filter_installed ----------------------------------------------------
    // `run()` requires an interactive TTY and calls std::process::exit, so it
    // cannot be unit-tested. We test `filter_installed` directly instead.

    #[test]
    fn known_unix_binary_is_detected() {
        // `ls` is present on every Unix system including macOS.
        let found = filter_installed(&["ls"]);
        assert_eq!(found, vec!["ls"]);
    }

    #[test]
    fn nonexistent_binary_is_excluded() {
        let found = filter_installed(&["__amnesia_nonexistent_binary_xyz__"]);
        assert!(found.is_empty());
    }

    #[test]
    fn mix_of_real_and_fake_keeps_only_real() {
        let found = filter_installed(&["ls", "__amnesia_fake__", "sh"]);
        assert!(found.contains(&"ls"));
        assert!(found.contains(&"sh"));
        assert!(!found.contains(&"__amnesia_fake__"));
    }

    #[test]
    fn empty_candidate_list_returns_empty() {
        let found = filter_installed(&[]);
        assert!(found.is_empty());
    }

    #[test]
    fn order_is_preserved() {
        // Both binaries exist; the result order must match input order.
        let found = filter_installed(&["sh", "ls"]);
        if found.len() == 2 {
            assert_eq!(found[0], "sh");
            assert_eq!(found[1], "ls");
        }
    }
}
