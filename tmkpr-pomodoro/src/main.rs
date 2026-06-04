mod app;
mod theme;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tmkpr_lib::{config::Config, storage::open_sqlite};

use app::{App, Screen};
use theme::Theme;

#[derive(Parser)]
#[command(name = "tmkpr-pomodoro", about = "Pomodoro timer for tmkpr")]
struct Args {
    #[arg(
        long,
        env = "TMKPR_THEME",
        help = "Colour theme: default, rose_pine, catppuccin_mocha, catppuccin_macchiato, \
                catppuccin_frappe, nord, gruvbox_dark, monokai, dracula, tokyonight, onedark, \
                solarized_dark, github_dark, kanagawa, everforest, ayu_dark"
    )]
    theme: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load()?;
    let db_path = config.database.path.clone();
    let storage = open_sqlite(&db_path)?;
    let user_id = config.user.user_id.clone();
    let theme_name = args.theme.as_deref().unwrap_or(&config.display.theme);
    let theme = Theme::resolve(theme_name, &config.themes);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, storage.as_ref(), &user_id, config, theme);

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
    theme: Theme,
) -> Result<()> {
    let mut app = App::new(storage, user_id, config, theme)?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.screen() {
                    Screen::Settings => {
                        if app.is_editing_sound() {
                            match key.code {
                                KeyCode::Char(c) => app.sound_edit_push(c),
                                KeyCode::Backspace => app.sound_edit_pop(),
                                KeyCode::Enter => app.sound_edit_confirm(),
                                KeyCode::Esc => app.sound_edit_cancel(),
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Up => app.settings_cursor_up(),
                                KeyCode::Down => app.settings_cursor_down(),
                                KeyCode::Left => app.settings_adjust(-1),
                                KeyCode::Right => app.settings_adjust(1),
                                KeyCode::Enter if app.settings_cursor_on_sound_field() => {
                                    app.sound_edit_begin();
                                }
                                KeyCode::Enter => app.settings_save()?,
                                KeyCode::Esc => app.settings_cancel(),
                                _ => {}
                            }
                        }
                    }
                    Screen::Main => {
                        if app.is_new_project_editing() {
                            match key.code {
                                KeyCode::Char(c) => app.new_project_push(c),
                                KeyCode::Backspace => app.new_project_pop(),
                                KeyCode::Enter => app.new_project_confirm()?,
                                KeyCode::Esc => app.new_project_cancel(),
                                _ => {}
                            }
                        } else if app.is_new_task_editing() {
                            match key.code {
                                KeyCode::Char(c) => app.new_task_push(c),
                                KeyCode::Backspace => app.new_task_pop(),
                                KeyCode::Enter => app.new_task_confirm()?,
                                KeyCode::Esc => app.new_task_cancel(),
                                _ => {}
                            }
                        } else if app.edit_mode() == app::EditMode::EditProject || app.edit_mode() == app::EditMode::EditTask {
                            match key.code {
                                KeyCode::Char(c) => app.edit_push(c),
                                KeyCode::Backspace => app.edit_pop(),
                                KeyCode::Enter => app.edit_confirm()?,
                                KeyCode::Esc => app.edit_cancel(),
                                _ => {}
                            }
                        } else if app.edit_mode() == app::EditMode::ConfirmDelete {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Enter => app.confirm_delete()?,
                                _ => app.cancel_delete(),
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc if app.can_quit() => {
                                    break;
                                }
                                KeyCode::Char('c') => app.task_complete_toggle()?,
                                KeyCode::Char('n') => app.new_task_begin(),
                                KeyCode::Char('P') => app.new_project_begin(),
                                KeyCode::Char('e') if !app.tasks().is_empty() => app.edit_task_begin(),
                                KeyCode::Char('e') => app.edit_project_begin(),
                                KeyCode::Char('d') if !app.tasks().is_empty() => app.delete_task_begin(),
                                KeyCode::Char('d') => app.delete_project_begin(),
                                KeyCode::Char('h') => app.toggle_hide_completed_tasks()?,
                                KeyCode::Char('+') => app.add_selected_task_to_queue(),
                                KeyCode::Char('-') => app.remove_from_queue(),
                                KeyCode::Char('{') => app.move_queue_up(),
                                KeyCode::Char('}') => app.move_queue_down(),
                                KeyCode::Char('[') => app.select_prev_queue(),
                                KeyCode::Char(']') => app.select_next_queue(),
                                KeyCode::Char('w') => app.start_queue_task()?,
                                KeyCode::Char('s') => app.open_settings(),
                                KeyCode::Up => app.previous_project(),
                                KeyCode::Down => app.next_project(),
                                KeyCode::Left => app.previous_task(),
                                KeyCode::Right => app.next_task(),
                                KeyCode::Enter => app.start_timer()?,
                                KeyCode::Char('b') => app.start_break()?,
                                KeyCode::Char(' ') => app.toggle_timer(),
                                KeyCode::Char('l') => app.log_session()?,
                                KeyCode::Char('r') => app.reset(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        app.update();
    }

    Ok(())
}
