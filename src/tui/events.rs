use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, LaunchStep, Tab};

pub fn handle(key: KeyEvent, app: &mut App) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    if app.input_buffer.is_some() {
        handle_input(key, app);
        return;
    }

    match app.active_tab {
        Tab::Launch => handle_launch(key, app),
        Tab::Databases => handle_databases(key, app),
        Tab::About => handle_about(key, app),
    }
}

fn handle_input(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char(c) => {
            if let Some(buf) = app.input_buffer.as_mut() {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = app.input_buffer.as_mut() {
                buf.pop();
            }
        }
        KeyCode::Enter => app.confirm_new_project(),
        KeyCode::Esc => {
            app.input_buffer = None;
        }
        _ => {}
    }
}

fn handle_launch(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Right => {
            app.active_tab = Tab::Databases;
            app.status_msg = None;
        }
        KeyCode::Esc => {
            if app.launch_step == LaunchStep::SelectProject {
                app.should_quit = true;
            } else {
                app.go_back();
            }
        }
        KeyCode::Char('q') => {
            if app.launch_step == LaunchStep::SelectProject {
                app.should_quit = true;
            }
        }
        KeyCode::Up => match app.launch_step {
            LaunchStep::SelectProject => {
                if app.project_cursor > 0 {
                    app.project_cursor -= 1;
                }
            }
            LaunchStep::SelectOrchestrator => {
                if app.orch_cursor > 0 {
                    app.orch_cursor -= 1;
                }
            }
            LaunchStep::SelectSession => {
                if app.session_cursor > 0 {
                    app.session_cursor -= 1;
                }
            }
        },
        KeyCode::Down => match app.launch_step {
            LaunchStep::SelectProject => {
                if app.project_cursor + 1 < app.project_items.len() {
                    app.project_cursor += 1;
                }
            }
            LaunchStep::SelectOrchestrator => {
                if app.orch_cursor + 1 < app.orch_items.len() {
                    app.orch_cursor += 1;
                }
            }
            LaunchStep::SelectSession => {
                if app.session_cursor + 1 < app.session_items.len() {
                    app.session_cursor += 1;
                }
            }
        },
        KeyCode::Enter => {
            app.status_msg = None;
            match app.launch_step {
                LaunchStep::SelectProject => app.confirm_project(),
                LaunchStep::SelectOrchestrator => app.confirm_orch(),
                LaunchStep::SelectSession => app.confirm_session(),
            }
        }
        _ => {}
    }
}

fn handle_databases(key: KeyEvent, app: &mut App) {
    if app.db_confirming {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_selected_db(),
            _ => {
                app.db_confirming = false;
                app.status_msg = None;
            }
        }
        return;
    }

    match key.code {
        KeyCode::Left => {
            app.active_tab = Tab::Launch;
            app.status_msg = None;
        }
        KeyCode::Right => {
            app.active_tab = Tab::About;
            app.status_msg = None;
        }
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up => {
            if app.db_cursor > 0 {
                app.db_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.db_cursor + 1 < app.db_entries.len() {
                app.db_cursor += 1;
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if !app.db_entries.is_empty() {
                let name = app.db_entries[app.db_cursor].name.clone();
                app.db_confirming = true;
                app.status_msg =
                    Some(format!("Delete '{name}'?  y = confirm  any other key = cancel"));
            }
        }
        _ => {}
    }
}

fn handle_about(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Left => {
            app.active_tab = Tab::Databases;
            app.status_msg = None;
        }
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEvent;

    use super::*;
    use crate::tui::app::{test_app, LaunchStep, Tab};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_c() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    }

    // --- Ctrl-C --------------------------------------------------------------

    #[test]
    fn ctrl_c_quits_on_launch_tab() {
        let mut app = test_app();
        handle(ctrl_c(), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_on_databases_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(ctrl_c(), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_while_in_input_mode() {
        let mut app = test_app();
        app.input_buffer = Some("typing".to_string());
        handle(ctrl_c(), &mut app);
        assert!(app.should_quit);
    }

    // --- Tab switching -------------------------------------------------------

    #[test]
    fn right_arrow_switches_to_databases_tab() {
        let mut app = test_app();
        handle(press(KeyCode::Right), &mut app);
        assert_eq!(app.active_tab, Tab::Databases);
    }

    #[test]
    fn left_arrow_switches_to_launch_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(press(KeyCode::Left), &mut app);
        assert_eq!(app.active_tab, Tab::Launch);
    }

    #[test]
    fn right_arrow_clears_status_msg() {
        let mut app = test_app();
        app.status_msg = Some("old".to_string());
        handle(press(KeyCode::Right), &mut app);
        assert!(app.status_msg.is_none());
    }

    // --- Launch tab: quit / esc ----------------------------------------------

    #[test]
    fn q_quits_at_project_step() {
        let mut app = test_app();
        handle(press(KeyCode::Char('q')), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn esc_quits_at_project_step() {
        let mut app = test_app();
        handle(press(KeyCode::Esc), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn q_does_not_quit_at_orchestrator_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        handle(press(KeyCode::Char('q')), &mut app);
        assert!(!app.should_quit);
    }

    #[test]
    fn esc_goes_back_at_orchestrator_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.selected_project = Some("alpha".to_string());
        handle(press(KeyCode::Esc), &mut app);
        assert!(!app.should_quit);
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    #[test]
    fn esc_goes_back_at_session_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        app.selected_orch = Some("claude".to_string());
        handle(press(KeyCode::Esc), &mut app);
        assert_eq!(app.launch_step, LaunchStep::SelectOrchestrator);
    }

    // --- Launch tab: cursor movement -----------------------------------------

    #[test]
    fn down_increments_project_cursor() {
        let mut app = test_app();
        app.project_cursor = 0;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.project_cursor, 1);
    }

    #[test]
    fn down_does_not_exceed_project_items_len() {
        let mut app = test_app();
        app.project_cursor = app.project_items.len() - 1;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.project_cursor, app.project_items.len() - 1);
    }

    #[test]
    fn up_decrements_project_cursor() {
        let mut app = test_app();
        app.project_cursor = 1;
        handle(press(KeyCode::Up), &mut app);
        assert_eq!(app.project_cursor, 0);
    }

    #[test]
    fn up_does_not_go_below_zero_for_project_cursor() {
        let mut app = test_app();
        app.project_cursor = 0;
        handle(press(KeyCode::Up), &mut app);
        assert_eq!(app.project_cursor, 0);
    }

    #[test]
    fn down_increments_orch_cursor() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.orch_cursor = 0;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.orch_cursor, 1);
    }

    #[test]
    fn down_does_not_exceed_orch_items_len() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.orch_cursor = app.orch_items.len() - 1;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.orch_cursor, app.orch_items.len() - 1);
    }

    #[test]
    fn down_increments_session_cursor() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        app.session_cursor = 0;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.session_cursor, 1);
    }

    #[test]
    fn up_does_not_go_below_zero_for_session_cursor() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        app.session_cursor = 0;
        handle(press(KeyCode::Up), &mut app);
        assert_eq!(app.session_cursor, 0);
    }

    // --- Launch tab: Enter ---------------------------------------------------

    #[test]
    fn enter_at_project_step_advances_to_orchestrator() {
        let mut app = test_app();
        app.project_cursor = 0; // "alpha"
        handle(press(KeyCode::Enter), &mut app);
        assert_eq!(app.launch_step, LaunchStep::SelectOrchestrator);
        assert_eq!(app.selected_project.as_deref(), Some("alpha"));
    }

    #[test]
    fn enter_on_new_project_item_sets_input_buffer() {
        let mut app = test_app();
        app.project_cursor = 2; // "  New project"
        handle(press(KeyCode::Enter), &mut app);
        assert_eq!(app.input_buffer, Some(String::new()));
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    #[test]
    fn enter_at_orch_step_advances_to_session() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        app.orch_cursor = 0;
        handle(press(KeyCode::Enter), &mut app);
        assert_eq!(app.launch_step, LaunchStep::SelectSession);
        assert_eq!(app.selected_orch.as_deref(), Some("claude"));
    }

    #[test]
    fn enter_at_session_step_existing_session_sets_launch_command() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        app.selected_project = Some("alpha".to_string());
        app.selected_orch = Some("claude".to_string());
        app.session_cursor = 1; // existing session, no disk write
        handle(press(KeyCode::Enter), &mut app);
        let cmd = app.launch_command.unwrap();
        assert_eq!(cmd.0, "claude");
        assert_eq!(cmd.1, "alpha");
        assert_eq!(cmd.2, "AAAABBBBCCCCDDDDEEEE000001");
    }

    // --- Input mode ----------------------------------------------------------

    #[test]
    fn input_mode_char_appended_to_buffer() {
        let mut app = test_app();
        app.input_buffer = Some("hel".to_string());
        handle(press(KeyCode::Char('l')), &mut app);
        assert_eq!(app.input_buffer.as_deref(), Some("hell"));
    }

    #[test]
    fn input_mode_backspace_pops_last_char() {
        let mut app = test_app();
        app.input_buffer = Some("hello".to_string());
        handle(press(KeyCode::Backspace), &mut app);
        assert_eq!(app.input_buffer.as_deref(), Some("hell"));
    }

    #[test]
    fn input_mode_backspace_on_empty_buffer_is_noop() {
        let mut app = test_app();
        app.input_buffer = Some(String::new());
        handle(press(KeyCode::Backspace), &mut app);
        assert_eq!(app.input_buffer.as_deref(), Some(""));
    }

    #[test]
    fn input_mode_esc_clears_buffer() {
        let mut app = test_app();
        app.input_buffer = Some("partial".to_string());
        handle(press(KeyCode::Esc), &mut app);
        assert!(app.input_buffer.is_none());
        assert!(!app.should_quit);
    }

    #[test]
    fn input_mode_enter_with_empty_buffer_does_not_advance() {
        let mut app = test_app();
        app.input_buffer = Some(String::new());
        handle(press(KeyCode::Enter), &mut app);
        assert_eq!(app.launch_step, LaunchStep::SelectProject);
    }

    #[test]
    fn input_mode_ignores_navigation_keys() {
        let mut app = test_app();
        app.input_buffer = Some("hi".to_string());
        handle(press(KeyCode::Down), &mut app);
        // project_cursor must not have moved
        assert_eq!(app.project_cursor, 0);
        assert_eq!(app.input_buffer.as_deref(), Some("hi"));
    }

    // --- Databases tab -------------------------------------------------------

    #[test]
    fn q_quits_on_databases_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(press(KeyCode::Char('q')), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn esc_quits_on_databases_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(press(KeyCode::Esc), &mut app);
        assert!(app.should_quit);
    }

    #[test]
    fn down_increments_db_cursor() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = 0;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.db_cursor, 1);
    }

    #[test]
    fn down_does_not_exceed_db_entries_len() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = app.db_entries.len() - 1;
        handle(press(KeyCode::Down), &mut app);
        assert_eq!(app.db_cursor, app.db_entries.len() - 1);
    }

    #[test]
    fn up_decrements_db_cursor() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = 1;
        handle(press(KeyCode::Up), &mut app);
        assert_eq!(app.db_cursor, 0);
    }

    #[test]
    fn up_does_not_go_below_zero_for_db_cursor() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = 0;
        handle(press(KeyCode::Up), &mut app);
        assert_eq!(app.db_cursor, 0);
    }

    #[test]
    fn d_key_sets_db_confirming_when_entries_present() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(press(KeyCode::Char('d')), &mut app);
        assert!(app.db_confirming);
        assert!(app.status_msg.is_some());
    }

    #[test]
    fn d_key_status_msg_contains_selected_name() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = 1; // "beta"
        handle(press(KeyCode::Char('d')), &mut app);
        assert!(app.status_msg.as_deref().unwrap().contains("beta"));
    }

    #[test]
    fn d_key_does_nothing_when_no_entries() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_entries.clear();
        handle(press(KeyCode::Char('d')), &mut app);
        assert!(!app.db_confirming);
    }

    #[test]
    fn delete_key_also_triggers_confirm() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        handle(press(KeyCode::Delete), &mut app);
        assert!(app.db_confirming);
    }

    #[test]
    fn non_y_in_confirming_cancels() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_confirming = true;
        app.status_msg = Some("Delete?".to_string());
        handle(press(KeyCode::Char('n')), &mut app);
        assert!(!app.db_confirming);
        assert!(app.status_msg.is_none());
    }

    #[test]
    fn uppercase_y_in_confirming_triggers_delete_state_update() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_cursor = 0;
        app.db_confirming = true;
        // delete_selected_db touches real disk but only removes non-existent paths,
        // so it's safe; we verify confirming is cleared afterward.
        handle(press(KeyCode::Char('Y')), &mut app);
        assert!(!app.db_confirming);
    }

    #[test]
    fn esc_does_not_quit_when_db_confirming() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_confirming = true;
        handle(press(KeyCode::Esc), &mut app);
        // Esc cancels confirming, does NOT quit
        assert!(!app.db_confirming);
        assert!(!app.should_quit);
    }
}
