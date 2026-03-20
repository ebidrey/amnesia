use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

// Standing pose sprite — 21×24
// 0=white  1=black  2=dark-blue  3=sky-blue  4=skin  5=transparent
const MEGAMAN: [[u8; 21]; 24] = [
    [5,5,5,5,5,5,5,5,5,5,1,1,1,5,5,5,5,5,5,5,5],
    [5,5,5,5,5,5,5,5,1,1,1,3,3,1,5,5,5,5,5,5,5],
    [5,5,5,5,5,5,5,1,2,2,2,1,3,3,1,5,5,5,5,5,5],
    [5,5,5,5,5,5,1,2,2,2,2,2,1,1,1,1,5,5,5,5,5],
    [5,5,5,5,5,5,1,2,2,2,2,2,1,3,3,2,1,5,5,5,5],
    [5,5,5,5,5,1,3,2,2,2,2,2,2,1,1,2,1,5,5,5,5],
    [5,5,5,5,5,1,3,2,2,4,0,0,0,2,2,0,1,5,5,5,5],
    [5,5,5,5,5,1,3,2,4,0,0,1,1,4,1,0,1,5,5,5,5],
    [5,5,5,5,5,5,1,2,4,0,0,1,1,4,1,0,1,5,5,5,5],
    [5,5,5,5,5,1,1,2,4,4,0,0,0,4,0,4,1,5,5,5,5],
    [5,5,5,1,1,3,3,1,2,4,1,1,1,1,4,1,1,1,5,5,5],
    [5,5,2,1,3,3,3,3,1,4,4,4,4,4,1,3,3,2,1,5,5],
    [5,5,1,2,2,3,3,3,3,1,1,1,1,1,3,3,2,2,1,5,5],
    [5,1,2,2,2,3,1,3,3,3,3,3,3,1,3,2,2,2,1,5,5],
    [5,1,2,2,1,1,1,3,3,3,3,3,3,1,1,1,2,2,1,5,5],
    [5,1,2,2,2,1,1,3,3,3,3,3,3,1,1,2,2,2,1,5,5],
    [5,1,2,2,2,1,1,2,2,2,2,2,2,1,1,2,2,2,1,5,5],
    [5,5,1,1,1,5,1,2,2,2,2,2,2,1,5,1,1,1,5,5,5],
    [5,5,5,5,5,1,3,3,2,2,2,2,2,3,3,1,5,5,5,5,5],
    [5,5,5,5,1,3,3,3,3,2,2,2,3,3,3,3,1,5,5,5,5],
    [5,5,5,1,1,2,2,2,3,3,1,3,3,2,2,2,1,1,5,5,5],
    [5,1,1,2,2,2,2,2,2,1,5,1,2,2,2,2,2,1,1,5,5],
    [1,2,2,2,2,2,2,2,1,5,5,5,1,2,2,2,2,2,2,1,5],
    [1,1,1,1,1,1,1,1,1,5,5,5,1,1,1,1,1,1,1,1,5],
];

fn pixel_color(n: u8) -> Color {
    match n {
        0 => Color::Rgb(255, 255, 255),
        1 => Color::Rgb(0, 0, 0),
        2 => Color::Rgb(43, 99, 195),
        3 => Color::Rgb(73, 203, 233),
        4 => Color::Rgb(241, 201, 163),
        _ => Color::Reset,
    }
}

fn draw_about(f: &mut Frame, area: Rect) {
    // Each pixel is 2 chars wide; art is 21×2=42 wide, 24 tall
    let art_w: u16 = 21 * 2;
    let art_h: u16 = 24;
    let x0 = area.x + area.width.saturating_sub(art_w) / 2;
    let y0 = area.y + area.height.saturating_sub(art_h) / 2;

    let buf = f.buffer_mut();
    for (row_i, row) in MEGAMAN.iter().enumerate() {
        let y = y0 + row_i as u16;
        if y >= area.bottom() {
            break;
        }
        for (col_i, &px) in row.iter().enumerate() {
            if px == 5 {
                continue;
            }
            let color = pixel_color(px);
            for dx in 0u16..2 {
                let x = x0 + col_i as u16 * 2 + dx;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol("█").set_fg(color);
                }
            }
        }
    }
}

use super::app::{App, LaunchStep, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_tabs(f, app, chunks[0]);

    match app.active_tab {
        Tab::Launch => draw_launch(f, app, chunks[1]),
        Tab::Databases => draw_databases(f, app, chunks[1]),
        Tab::About => draw_about(f, chunks[1]),
    }

    draw_status(f, app, chunks[2]);

    if app.input_buffer.is_some() {
        draw_input_popup(f, app, area);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![Line::from("Launch"), Line::from("Databases"), Line::from("About")];
    let selected = match app.active_tab {
        Tab::Launch => 0usize,
        Tab::Databases => 1,
        Tab::About => 2,
    };
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(tabs, area);
}

fn draw_launch(f: &mut Frame, app: &mut App, area: Rect) {
    let title = match app.launch_step {
        LaunchStep::SelectProject => "Select project",
        LaunchStep::SelectOrchestrator => "Select orchestrator",
        LaunchStep::SelectSession => "Select session",
    };
    let cursor = match app.launch_step {
        LaunchStep::SelectProject => app.project_cursor,
        LaunchStep::SelectOrchestrator => app.orch_cursor,
        LaunchStep::SelectSession => app.session_cursor,
    };
    let list_items: Vec<ListItem> = match app.launch_step {
        LaunchStep::SelectProject => {
            app.project_items.iter().map(|s| ListItem::new(s.clone())).collect()
        }
        LaunchStep::SelectOrchestrator => {
            app.orch_items.iter().map(|s| ListItem::new(s.clone())).collect()
        }
        LaunchStep::SelectSession => {
            app.session_items.iter().map(|s| ListItem::new(s.clone())).collect()
        }
    };

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("→ ");

    let mut state = ListState::default();
    state.select(Some(cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_databases(f: &mut Frame, app: &mut App, area: Rect) {
    if app.db_entries.is_empty() {
        let p = Paragraph::new("No project databases found.")
            .block(Block::default().borders(Borders::ALL).title("Databases"));
        f.render_widget(p, area);
        return;
    }

    let list_items: Vec<ListItem> = app
        .db_entries
        .iter()
        .map(|e| {
            let text = format!("  {}  ({} observations)", e.name, e.observation_count);
            ListItem::new(text)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("Databases  [d] delete"))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("→ ");

    let mut state = ListState::default();
    state.select(Some(app.db_cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if let Some(msg) = &app.status_msg {
        (msg.as_str(), Style::default().fg(Color::Yellow))
    } else {
        let hint = match app.active_tab {
            Tab::Launch => "↑↓ navigate  Enter select  Esc back  → Databases  q quit",
            Tab::Databases => "↑↓ navigate  d delete  ← Launch  → About  q quit",
            Tab::About => "← Databases  q quit",
        };
        (hint, Style::default().fg(Color::DarkGray))
    };
    f.render_widget(Paragraph::new(text).style(style), area);
}

fn draw_input_popup(f: &mut Frame, app: &App, area: Rect) {
    let buffer = app.input_buffer.as_deref().unwrap_or("");
    let popup = centered_rect(60, 5, area);
    f.render_widget(Clear, popup);

    let text = format!("New project name: {}_", buffer);
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Create project")
                .style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(para, popup);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: popup_width.min(r.width),
        height: height.min(r.height),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;
    use crate::tui::app::{test_app, LaunchStep, Tab};

    /// Render to a 100×30 TestBackend and return all cell symbols concatenated.
    fn render(app: &mut App) -> String {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal.backend().buffer().content().iter().map(|c| c.symbol()).collect()
    }

    // --- Tabs bar ------------------------------------------------------------

    #[test]
    fn launch_tab_label_is_rendered() {
        let mut app = test_app();
        assert!(render(&mut app).contains("Launch"));
    }

    #[test]
    fn databases_tab_label_is_rendered() {
        let mut app = test_app();
        assert!(render(&mut app).contains("Databases"));
    }

    #[test]
    fn both_tab_labels_visible_regardless_of_active_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        let out = render(&mut app);
        assert!(out.contains("Launch"));
        assert!(out.contains("Databases"));
    }

    // --- Launch tab content --------------------------------------------------

    #[test]
    fn launch_tab_shows_select_project_title() {
        let mut app = test_app();
        assert!(render(&mut app).contains("Select project"));
    }

    #[test]
    fn launch_tab_shows_project_names() {
        let mut app = test_app();
        let out = render(&mut app);
        assert!(out.contains("alpha"));
        assert!(out.contains("beta"));
    }

    #[test]
    fn launch_tab_shows_orchestrator_title_at_orch_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        assert!(render(&mut app).contains("Select orchestrator"));
    }

    #[test]
    fn launch_tab_shows_orch_names() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectOrchestrator;
        let out = render(&mut app);
        assert!(out.contains("claude"));
        assert!(out.contains("opencode"));
    }

    #[test]
    fn launch_tab_shows_session_title_at_session_step() {
        let mut app = test_app();
        app.launch_step = LaunchStep::SelectSession;
        assert!(render(&mut app).contains("Select session"));
    }

    // --- Databases tab content -----------------------------------------------

    #[test]
    fn databases_tab_shows_project_names() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        let out = render(&mut app);
        assert!(out.contains("alpha"));
        assert!(out.contains("beta"));
    }

    #[test]
    fn databases_tab_shows_observation_counts() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        let out = render(&mut app);
        assert!(out.contains("5 observations"));
        assert!(out.contains("0 observations"));
    }

    #[test]
    fn databases_tab_shows_delete_hint_in_title() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        assert!(render(&mut app).contains("[d] delete"));
    }

    #[test]
    fn databases_tab_empty_shows_no_databases_message() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        app.db_entries.clear();
        assert!(render(&mut app).contains("No project databases found"));
    }

    // --- Status bar ----------------------------------------------------------

    #[test]
    fn status_bar_shows_launch_hint_on_launch_tab() {
        let mut app = test_app();
        let out = render(&mut app);
        assert!(out.contains("Enter select"));
    }

    #[test]
    fn status_bar_shows_databases_hint_on_databases_tab() {
        let mut app = test_app();
        app.active_tab = Tab::Databases;
        let out = render(&mut app);
        assert!(out.contains("d delete"));
    }

    #[test]
    fn status_msg_overrides_hint() {
        let mut app = test_app();
        app.status_msg = Some("custom status here".to_string());
        let out = render(&mut app);
        assert!(out.contains("custom status here"));
        assert!(!out.contains("Enter select"));
    }

    // --- Input popup ---------------------------------------------------------

    #[test]
    fn input_popup_rendered_when_input_buffer_set() {
        let mut app = test_app();
        app.input_buffer = Some("myproject".to_string());
        let out = render(&mut app);
        assert!(out.contains("Create project"));
        assert!(out.contains("myproject"));
    }

    #[test]
    fn input_popup_not_rendered_without_input_buffer() {
        let mut app = test_app();
        app.input_buffer = None;
        assert!(!render(&mut app).contains("Create project"));
    }
}
