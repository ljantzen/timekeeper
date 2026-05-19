mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tmkpr_lib::{config::Config, storage::open_sqlite};

use app::{App, Screen};

fn main() -> Result<()> {
    // Setup
    let config = Config::load()?;
    let db_path = config.database.path.clone();
    let storage = open_sqlite(&db_path)?;
    let user_id = config.user.user_id.clone();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, storage.as_ref(), &user_id, config);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    app_result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    storage: &dyn tmkpr_lib::storage::Storage,
    user_id: &str,
    config: tmkpr_lib::config::Config,
) -> Result<()> {
    let mut app = App::new(storage, user_id, config)?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.screen() {
                    Screen::Settings => match key.code {
                        KeyCode::Up => app.settings_cursor_up(),
                        KeyCode::Down => app.settings_cursor_down(),
                        KeyCode::Left => app.settings_adjust(-1),
                        KeyCode::Right => app.settings_adjust(1),
                        KeyCode::Enter => app.settings_save()?,
                        KeyCode::Esc => app.settings_cancel(),
                        _ => {}
                    },
                    Screen::Main => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc
                            if app.can_quit() => {
                                break;
                            }
                        KeyCode::Char('s') => app.open_settings(),
                        KeyCode::Up => app.previous_project(),
                        KeyCode::Down => app.next_project(),
                        KeyCode::Left => app.previous_task(),
                        KeyCode::Right => app.next_task(),
                        KeyCode::Enter => app.start_timer()?,
                        KeyCode::Char(' ') => app.toggle_timer(),
                        KeyCode::Char('l') => app.log_session()?,
                        KeyCode::Char('r') => app.reset(),
                        _ => {}
                    },
                }
            }
        }

        app.update();
    }

    Ok(())
}
