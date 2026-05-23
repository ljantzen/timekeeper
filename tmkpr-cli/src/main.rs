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

use cli::{Cli, Commands, CommentCommands, ProjectCommands, TaskCommands};

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();
    if let Err(e) = run() {
        error::print_error(&e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = Config::load()?;
    let color = !cli.no_color && config.display.color;
    let date_fmt = config.display.date_format.clone();
    let time_fmt = config.display.time_format;
    let user_id = config.user.user_id.clone();
    let format = cli.format.as_str();

    let db_path = cli.db.unwrap_or(config.database.path);
    let storage = open_sqlite(&db_path)?;

    match cli.command.unwrap_or(Commands::Status) {
        Commands::Start(args) => {
            commands::track::run(args, storage.as_ref(), &user_id, &date_fmt, time_fmt, color)?
        }
        Commands::Stop(args) => {
            commands::stop::run(args, storage.as_ref(), &user_id, &date_fmt, time_fmt)?
        }
        Commands::Log(args) => {
            commands::log::run(args, storage.as_ref(), &user_id, &date_fmt, time_fmt, color)?
        }
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
            ProjectCommands::Add(args) => commands::project::add(args, storage.as_ref(), &user_id)?,
            ProjectCommands::List(args) => {
                commands::project::list(args, storage.as_ref(), &user_id, format, color)?
            }
            ProjectCommands::Edit(args) => {
                commands::project::edit(args, storage.as_ref(), &user_id)?
            }
            ProjectCommands::Delete(args) => {
                commands::project::delete(args, storage.as_ref(), &user_id)?
            }
        },
        Commands::Task(sub) => match sub {
            TaskCommands::Add(args) => commands::task::add(args, storage.as_ref(), &user_id)?,
            TaskCommands::List(args) => {
                commands::task::list(args, storage.as_ref(), &user_id, format)?
            }
            TaskCommands::Edit(args) => commands::task::edit(args, storage.as_ref(), &user_id)?,
            TaskCommands::Delete(args) => commands::task::delete(args, storage.as_ref(), &user_id)?,
            TaskCommands::Done(args) => commands::task::done(args, storage.as_ref(), &user_id)?,
            TaskCommands::Reactivate(args) => {
                commands::task::reactivate(args, storage.as_ref(), &user_id)?
            }
        },
        Commands::Edit(args) => {
            commands::edit::run(args, storage.as_ref(), &user_id, &date_fmt, time_fmt, color)?
        }
        Commands::Delete(args) => commands::delete::run(args, storage.as_ref(), &user_id)?,
        Commands::Merge(args) => commands::merge::run(args, storage.as_ref(), &user_id)?,
        Commands::FillGap(args) => commands::fill_gap::run(args, storage.as_ref(), &user_id)?,
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
    }

    Ok(())
}
