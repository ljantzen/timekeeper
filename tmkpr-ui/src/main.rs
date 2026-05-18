mod app;
mod form;
mod input;
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

#[derive(Parser)]
#[command(name = "tmkpr-ui", about = "Terminal UI for tmkpr")]
struct Args {
    #[arg(long, env = "TMKPR_DB", help = "Database path override")]
    db: Option<std::path::PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load()?;
    let user_id = config.user.user_id.clone();
    let db_path = args.db.unwrap_or(config.database.path);
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

    let result = run_app(&mut terminal, storage, user_id);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    storage: Box<dyn tmkpr_lib::storage::Storage>,
    user_id: String,
) -> anyhow::Result<()> {
    let mut app = App::new(storage, user_id);
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

        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
        }

        if !app.running {
            break;
        }
    }

    let _ = app.save_ui_state();
    Ok(())
}
