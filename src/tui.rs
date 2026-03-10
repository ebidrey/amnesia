mod app;
mod events;
mod ui;

use std::io;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = app::App::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                events::handle(key, &mut app);
            }
        }

        if app.should_quit {
            break;
        }

        if let Some((orch, project, session_id)) = app.launch_command.take() {
            // Restore terminal before handing control to the orchestrator
            let _ = disable_raw_mode();
            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
            let _ = terminal.show_cursor();

            eprintln!("-> Launching {orch}...");
            let status = std::process::Command::new(&orch)
                .env("AMNESIA_PROJECT", &project)
                .env("AMNESIA_SESSION", &session_id)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}
