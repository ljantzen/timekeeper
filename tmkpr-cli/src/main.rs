mod cli;
mod commands;
mod completers;
mod error;
mod output;
mod prompt;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use tmkpr_lib::config::Config;
use tmkpr_lib::storage::open_sqlite;

use cli::{Cli, Commands, CommentCommands, EventCommands, ProjectCommands, TaskCommands};

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();
    if let Err(e) = run() {
        error::print_error(&e);
        std::process::exit(1);
    }
}

fn launch(binary: &str, db: Option<&std::path::Path>, extra: &[String]) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(binary);
    if let Some(path) = db {
        cmd.arg("--db").arg(path);
    }
    cmd.args(extra);
    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "'{binary}' not found — is it installed? (`cargo install --path {}`)",
                binary.replace('-', "/")
            )
        } else {
            anyhow::anyhow!("failed to launch '{binary}': {e}")
        }
    })?;
    std::process::exit(status.code().unwrap_or(1));
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle launch subcommands before opening the database.
    let db_override = cli.db.as_deref();
    match &cli.command {
        Some(Commands::Ui(args)) => return launch("tmkpr-ui", db_override, &args.args),
        Some(Commands::Pomodoro(args)) => return launch("tmkpr-pomodoro", db_override, &args.args),
        _ => {}
    }

    let config = Config::load()?;
    let color = !cli.no_color && config.display.color;
    let date_fmt = config.display.date_format.clone();
    let time_fmt = config.display.time_format;
    let user_id = config.user.user_id.clone();
    let format = cli.format.as_str();

    let db_path = cli.db.unwrap_or_else(|| config.database.path.clone());
    let storage = open_sqlite(&db_path)?;

    match cli.command.unwrap_or(Commands::Status) {
        Commands::Start(args) => commands::track::run(
            args,
            storage.as_ref(),
            &user_id,
            &date_fmt,
            time_fmt,
            color,
            &config,
        )?,
        Commands::Stop(args) => commands::stop::run(
            args,
            storage.as_ref(),
            &user_id,
            &date_fmt,
            time_fmt,
            &config,
        )?,
        Commands::Log(args) => commands::log::run(
            args,
            storage.as_ref(),
            &user_id,
            &date_fmt,
            time_fmt,
            color,
            &config,
        )?,
        Commands::Status => {
            commands::status::run(storage.as_ref(), &user_id, &date_fmt, format, color)?
        }
        Commands::List(args) => commands::list::run(
            args,
            storage.as_ref(),
            &user_id,
            &date_fmt,
            time_fmt,
            format,
            color,
        )?,
        Commands::Report(args) => {
            commands::report::run(args, storage.as_ref(), &user_id, time_fmt, format, color)?
        }
        Commands::Project(sub) => match sub {
            ProjectCommands::Add(args) => {
                commands::project::add(args, storage.as_ref(), &user_id, &config)?
            }
            ProjectCommands::List(args) => {
                commands::project::list(args, storage.as_ref(), &user_id, format, color)?
            }
            ProjectCommands::Edit(args) => {
                commands::project::edit(args, storage.as_ref(), &user_id, &config)?
            }
            ProjectCommands::Delete(args) => {
                commands::project::delete(args, storage.as_ref(), &user_id, &config)?
            }
        },
        Commands::Task(sub) => match sub {
            TaskCommands::Add(args) => {
                commands::task::add(args, storage.as_ref(), &user_id, &config)?
            }
            TaskCommands::List(args) => {
                commands::task::list(args, storage.as_ref(), &user_id, format)?
            }
            TaskCommands::Edit(args) => {
                commands::task::edit(args, storage.as_ref(), &user_id, &config)?
            }
            TaskCommands::Delete(args) => {
                commands::task::delete(args, storage.as_ref(), &user_id, &config)?
            }
            TaskCommands::Done(args) => {
                commands::task::done(args, storage.as_ref(), &user_id, &config)?
            }
            TaskCommands::Reactivate(args) => {
                commands::task::reactivate(args, storage.as_ref(), &user_id, &config)?
            }
        },
        Commands::Edit(args) => {
            commands::edit::run(args, storage.as_ref(), &user_id, &date_fmt, time_fmt, color)?
        }
        Commands::Delete(args) => commands::delete::run(args, storage.as_ref(), &user_id, &config)?,
        Commands::Merge(args) => commands::merge::run(args, storage.as_ref(), &user_id, &config)?,
        Commands::FillGap(args) => commands::fill_gap::run(args, storage.as_ref(), &user_id)?,
        Commands::Event(sub) => match sub {
            EventCommands::Add(args) => commands::event::add(
                args,
                storage.as_ref(),
                &user_id,
                &date_fmt,
                time_fmt,
                color,
                &config,
            )?,
            EventCommands::List(args) => commands::event::list(
                args,
                storage.as_ref(),
                &user_id,
                &date_fmt,
                time_fmt,
                format,
                color,
            )?,
            EventCommands::Edit(args) => commands::event::edit(
                args,
                storage.as_ref(),
                &user_id,
                &date_fmt,
                time_fmt,
                color,
                &config,
            )?,
            EventCommands::Delete(args) => {
                commands::event::delete(args, storage.as_ref(), &user_id, &config)?
            }
        },
        Commands::Comment(sub) => match sub {
            CommentCommands::Add(args) => commands::comment::add(args, storage.as_ref(), &user_id)?,
            CommentCommands::List(args) => {
                commands::comment::list(args, storage.as_ref(), &user_id, &date_fmt, format)?
            }
            CommentCommands::Edit(args) => {
                commands::comment::edit(args, storage.as_ref(), &user_id)?
            }
            CommentCommands::Delete(args) => {
                commands::comment::delete(args, storage.as_ref(), &user_id)?
            }
        },
        Commands::Completion(args) => commands::completion::run(args)?,
        Commands::Import(args) => commands::import::run(args, storage.as_ref(), &user_id, format)?,
        Commands::Export(args) => {
            commands::export::run(args, storage.as_ref(), &user_id, time_fmt, format)?
        }
        // Already handled above before storage is opened.
        Commands::Ui(_) | Commands::Pomodoro(_) => unreachable!(),
    }

    Ok(())
}
