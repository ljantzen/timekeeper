mod app;
mod color;
mod form;
mod input;
mod theme;
mod ui;

use std::io::Stdout;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::{
    event::{Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tmkpr_lib::{config::Config, storage::open_sqlite};

use app::App;
use theme::Theme;

#[derive(Parser)]
#[command(name = "tmkpr-ui", about = "Terminal UI for tmkpr")]
struct Args {
    #[arg(long, env = "TMKPR_DB", help = "Database path override")]
    db: Option<std::path::PathBuf>,
    #[arg(
        long,
        env = "TMKPR_THEME",
        help = "Colour theme: default, rose_pine, catppuccin_mocha, catppuccin_macchiato, \
                catppuccin_frappe, nord, gruvbox_dark, monokai, dracula, tokyonight, onedark, \
                solarized_dark, github_dark, kanagawa, everforest, ayu_dark"
    )]
    theme: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load()?;
    let user_id = config.user.user_id.clone();
    let db_path = args.db.unwrap_or(config.database.path.clone());
    let theme_name = args.theme.as_deref().unwrap_or(&config.display.theme);
    let theme = Theme::resolve(theme_name, &config.themes);
    let storage = open_sqlite(&db_path)?;

    // Restore terminal on panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stderr(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(
        storage,
        user_id,
        theme_name.to_string(),
        theme,
        config.themes.clone(),
        config.display.date_format.clone(),
        chrono::Weekday::from(config.display.week_start),
        config.clone(),
    );
    let result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> anyhow::Result<()> {
    app.refresh()?;
    app.load_ui_state()?;
    app.status = None;

    let tick = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        let timeout = tick.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    input::handle_key(&mut app, key)?;
                }
            }
        }

        if let Some(path) = app.pending_open.take() {
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "notepad.exe".to_string()
                    } else {
                        "vi".to_string()
                    }
                });
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            std::process::Command::new(&editor).arg(&path).status()?;
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?;
            // Auto-reload config after editing
            if let Ok(cfg) = tmkpr_lib::config::Config::load() {
                let name = cfg.display.theme.clone();
                app.themes = cfg.themes.clone();
                app.theme = Theme::resolve(&name, &app.themes);
                app.theme_name = name;
                app.date_format = cfg.display.date_format.clone();
                app.week_start = chrono::Weekday::from(cfg.display.week_start);
                app.status_timeout_secs = cfg.display.status_timeout_secs;
            }
            app.set_status("Config reloaded.".to_string(), false);
        }

        if last_tick.elapsed() >= tick {
            app.tick_status();
            last_tick = Instant::now();
        }

        if !app.running {
            break;
        }
    }

    let _ = app.save_ui_state();
    Ok(())
}
