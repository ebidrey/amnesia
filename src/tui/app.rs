use chrono::Utc;
use ulid::Ulid;

use crate::{config, model::Session, projects, sessions};

const KNOWN_ORCHESTRATORS: &[&str] = &["claude", "opencode", "cursor", "aider", "goose"];

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    Launch,
    Databases,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LaunchStep {
    SelectProject,
    SelectOrchestrator,
    SelectSession,
}

pub struct DbEntry {
    pub name: String,
    pub observation_count: usize,
}

pub struct App {
    pub active_tab: Tab,

    // Launch tab
    pub launch_step: LaunchStep,
    pub project_items: Vec<String>,
    pub project_cursor: usize,
    pub selected_project: Option<String>,
    pub orch_items: Vec<String>,
    pub orch_cursor: usize,
    pub selected_orch: Option<String>,
    pub session_items: Vec<String>,
    pub session_cursor: usize,
    pub sessions_raw: Vec<Session>,
    pub input_buffer: Option<String>, // Some(_) = typing new project name

    // Databases tab
    pub db_entries: Vec<DbEntry>,
    pub db_cursor: usize,
    pub db_confirming: bool,

    // Control
    pub should_quit: bool,
    pub launch_command: Option<(String, String, String)>, // (orch, project, session_id)
    pub status_msg: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let projects_cfg = projects::load();
        let mut project_items: Vec<String> =
            projects_cfg.projects.iter().map(|p| p.name.clone()).collect();
        project_items.push("  New project".to_string());

        let orch_items: Vec<String> = KNOWN_ORCHESTRATORS
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
            .map(|s| s.to_string())
            .collect();

        let db_entries = Self::build_db_entries();

        App {
            active_tab: Tab::Launch,
            launch_step: LaunchStep::SelectProject,
            project_items,
            project_cursor: 0,
            selected_project: None,
            orch_items,
            orch_cursor: 0,
            selected_orch: None,
            session_items: vec![],
            session_cursor: 0,
            sessions_raw: vec![],
            input_buffer: None,
            db_entries,
            db_cursor: 0,
            db_confirming: false,
            should_quit: false,
            launch_command: None,
            status_msg: None,
        }
    }

    pub fn build_db_entries() -> Vec<DbEntry> {
        let projects_cfg = projects::load();
        projects_cfg
            .projects
            .iter()
            .map(|p| {
                let store_path = config::project_store_path(&p.name);
                let observation_count = if store_path.exists() {
                    std::fs::read_to_string(store_path)
                        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                        .unwrap_or(0)
                } else {
                    0
                };
                DbEntry { name: p.name.clone(), observation_count }
            })
            .collect()
    }

    pub fn refresh_db_entries(&mut self) {
        self.db_entries = Self::build_db_entries();
        if !self.db_entries.is_empty() && self.db_cursor >= self.db_entries.len() {
            self.db_cursor = self.db_entries.len() - 1;
        }
    }

    pub fn load_sessions_for(&mut self, project: &str) {
        let sessions_path = config::project_sessions_path(project);
        let mut existing = sessions::load_from(&sessions_path).unwrap_or_default();
        existing.sort_by(|a, b| b.id.cmp(&a.id));
        existing.truncate(5);

        let mut items = vec!["  New session".to_string()];
        for s in &existing {
            let date = &s.started_at[..16].replace('T', " ");
            items.push(format!("{}  {}  {}", date, s.orchestrator, &s.id[..8]));
        }
        self.sessions_raw = existing;
        self.session_items = items;
        self.session_cursor = 0;
    }

    pub fn confirm_project(&mut self) {
        let selected = self.project_items[self.project_cursor].trim().to_string();
        if selected == "New project" {
            self.input_buffer = Some(String::new());
            return;
        }
        let name = selected.clone();
        self.load_sessions_for(&name);
        self.selected_project = Some(name);
        self.launch_step = LaunchStep::SelectOrchestrator;
        self.status_msg = None;
    }

    pub fn confirm_new_project(&mut self) {
        let Some(name) = self.input_buffer.take() else { return };
        if name.is_empty() {
            return;
        }
        let mut cfg = projects::load();
        cfg.projects.push(projects::Project { name: name.clone() });
        let _ = projects::save(&cfg);

        let insert_pos = self.project_items.len() - 1;
        self.project_items.insert(insert_pos, name.clone());
        self.project_cursor = insert_pos;

        self.load_sessions_for(&name);
        self.selected_project = Some(name);
        self.launch_step = LaunchStep::SelectOrchestrator;
        self.status_msg = None;
    }

    pub fn confirm_orch(&mut self) {
        if self.orch_items.is_empty() {
            self.status_msg = Some(
                "No orchestrators found. Install claude, opencode, cursor, aider or goose."
                    .to_string(),
            );
            return;
        }
        let orch = self.orch_items[self.orch_cursor].clone();
        self.selected_orch = Some(orch);
        self.launch_step = LaunchStep::SelectSession;
        self.status_msg = None;
    }

    pub fn confirm_session(&mut self) {
        let project = self.selected_project.clone().unwrap_or_default();
        let orch = self.selected_orch.clone().unwrap_or_default();

        let session_id = if self.session_cursor == 0 {
            let id = Ulid::new().to_string();
            let session = Session {
                id: id.clone(),
                project: project.clone(),
                orchestrator: orch.clone(),
                started_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            };
            let sessions_path = config::project_sessions_path(&project);
            let _ = sessions::append_to(&sessions_path, &session);
            id
        } else {
            self.sessions_raw[self.session_cursor - 1].id.clone()
        };

        self.launch_command = Some((orch, project, session_id));
    }

    pub fn go_back(&mut self) {
        match self.launch_step {
            LaunchStep::SelectProject => {}
            LaunchStep::SelectOrchestrator => {
                self.launch_step = LaunchStep::SelectProject;
                self.selected_project = None;
            }
            LaunchStep::SelectSession => {
                self.launch_step = LaunchStep::SelectOrchestrator;
                self.selected_orch = None;
            }
        }
        self.status_msg = None;
    }

    pub fn delete_selected_db(&mut self) {
        if self.db_entries.is_empty() {
            return;
        }
        let name = self.db_entries[self.db_cursor].name.clone();

        let mut cfg = projects::load();
        cfg.projects.retain(|p| p.name != name);
        let _ = projects::save(&cfg);

        let dir = config::projects_dir().join(&name);
        let _ = std::fs::remove_dir_all(&dir);

        self.refresh_db_entries();
        self.db_confirming = false;

        // Sync launch tab project list
        let mut items: Vec<String> =
            cfg.projects.iter().map(|p| p.name.clone()).collect();
        items.push("  New project".to_string());
        self.project_items = items;
        if self.project_cursor >= self.project_items.len().saturating_sub(1) {
            self.project_cursor = self.project_items.len().saturating_sub(1);
        }

        self.status_msg = Some(format!("Deleted database '{name}'"));
    }
}

/// Shared test fixture — available to all tests inside `src/tui/`.
#[cfg(test)]
pub(crate) fn test_app() -> App {
    App {
        active_tab: Tab::Launch,
        launch_step: LaunchStep::SelectProject,
        project_items: vec![
            "alpha".to_string(),
            "beta".to_string(),
            "  New project".to_string(),
        ],
        project_cursor: 0,
        selected_project: None,
        orch_items: vec!["claude".to_string(), "opencode".to_string()],
        orch_cursor: 0,
        selected_orch: None,
        session_items: vec![
            "  New session".to_string(),
            "2026-03-08 22:05  claude  AAAABBBB".to_string(),
        ],
        session_cursor: 0,
        sessions_raw: vec![Session {
            id: "AAAABBBBCCCCDDDDEEEE000001".to_string(),
            project: "alpha".to_string(),
            orchestrator: "claude".to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        }],
        input_buffer: None,
        db_entries: vec![
            DbEntry { name: "alpha".to_string(), observation_count: 5 },
            DbEntry { name: "beta".to_string(), observation_count: 0 },
        ],
        db_cursor: 0,
        db_confirming: false,
        should_quit: false,
        launch_command: None,
        status_msg: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- go_back -------------------------------------------------------------

    #[test]
    fn go_back_from_select_project_leaves_step_unchanged() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectProject;
        app.go_back();
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    #[test]
    fn go_back_from_orchestrator_returns_to_project_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.selected_project = Some("alpha".to_string());
        app.go_back();
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
        assert!(app.selected_project.is_none());
    }

    #[test]
    fn go_back_from_session_returns_to_orchestrator_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        app.selected_orch = Some("claude".to_string());
        app.go_back();
        assert_eq!(app.launch_step, LaunchStep::SelectOrchestrator);
        assert!(app.selected_orch.is_none());
    }

    #[test]
    fn go_back_clears_status_message() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.status_msg = Some("something".to_string());
        app.go_back();
        assert!(app.status_msg.is_none());
    }

    // --- confirm_orch --------------------------------------------------------

    #[test]
    fn confirm_orch_with_no_orchestrators_sets_status_message() {
        let mut app = test_app();
        app.orch_items.clear();
        app.confirm_orch();
        assert!(app.status_msg.is_some());
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    #[test]
    fn confirm_orch_advances_to_session_step() {
        let mut app = test_app();
        app.orch_cursor = 0;
        app.confirm_orch();
        assert_eq!(app.launch_step, LaunchStep::SelectSession);
        assert_eq!(app.selected_orch.as_deref(), Some("claude"));
    }

    #[test]
    fn confirm_orch_second_item() {
        let mut app = test_app();
        app.orch_cursor = 1;
        app.confirm_orch();
        assert_eq!(app.selected_orch.as_deref(), Some("opencode"));
    }

    #[test]
    fn confirm_orch_clears_status_message() {
        let mut app = test_app();
        app.status_msg = Some("old message".to_string());
        app.confirm_orch();
        assert!(app.status_msg.is_none());
    }

    // --- confirm_project -----------------------------------------------------

    #[test]
    fn confirm_project_new_project_item_sets_input_buffer() {
        let mut app = test_app();
        // "  New project" is at index 2
        app.project_cursor = 2;
        app.confirm_project();
        assert_eq!(app.input_buffer, Some(String::new()));
        assert_eq!(app.launch_step, LaunchStep::SelectProject); // didn't advance
    }

    #[test]
    fn confirm_project_real_item_advances_step() {
        let mut app = test_app();
        app.project_cursor = 0; // "alpha"
        app.confirm_project();
        assert_eq!(app.launch_step, LaunchStep::SelectOrchestrator);
        assert_eq!(app.selected_project.as_deref(), Some("alpha"));
        assert!(app.input_buffer.is_none());
    }

    // --- confirm_new_project -------------------------------------------------

    #[test]
    fn confirm_new_project_with_empty_buffer_is_noop() {
        let mut app = test_app();
        app.input_buffer = Some(String::new());
        let items_before = app.project_items.len();
        app.confirm_new_project();
        // input_buffer is consumed but launch_step must not advance
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
        assert_eq!(app.project_items.len(), items_before);
    }

    #[test]
    fn confirm_new_project_with_none_buffer_is_noop() {
        let mut app = test_app();
        app.input_buffer = None;
        app.confirm_new_project();
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    // --- confirm_session -----------------------------------------------------

    #[test]
    fn confirm_session_existing_session_sets_launch_command() {
        let mut app = test_app();
        app.selected_project = Some("alpha".to_string());
        app.selected_orch = Some("claude".to_string());
        app.session_cursor = 1; // pick existing session, no disk write
        app.confirm_session();
        let cmd = app.launch_command.unwrap();
        assert_eq!(cmd.0, "claude");
        assert_eq!(cmd.1, "alpha");
        assert_eq!(cmd.2, "AAAABBBBCCCCDDDDEEEE000001");
    }

    // --- refresh_db_entries --------------------------------------------------

    #[test]
    fn refresh_db_entries_clamps_cursor_when_out_of_bounds() {
        let mut app = test_app();
        // Manually shrink entries without going through delete
        app.db_entries.truncate(1);
        app.db_cursor = 5; // way out of bounds
        app.refresh_db_entries();
        // After refresh, build_db_entries reads real disk; whatever it returns,
        // cursor must be within bounds.
        if !app.db_entries.is_empty() {
            assert!(app.db_cursor < app.db_entries.len());
        }
    }
}
