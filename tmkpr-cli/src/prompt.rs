use std::io::{self, IsTerminal, Write};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use tmkpr_lib::error::TmkprError;
use tmkpr_lib::models::{project::Project, task::Task};
use tmkpr_lib::service::{ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

pub fn confirm(question: &str) -> bool {
    print!("{} [y/N] ", question);
    io::stdout().flush().ok();

    // If stdin isn't a TTY (e.g. piped input), fall back to line-buffered read.
    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        return matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
    }

    terminal::enable_raw_mode().ok();
    let accepted = 'outer: loop {
        if let Ok(Event::Key(key)) = event::read() {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if key.code == KeyCode::Char('c') {
                    terminal::disable_raw_mode().ok();
                    println!();
                    std::process::exit(130);
                }
                continue;
            }
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => break 'outer true,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                    break 'outer false
                }
                _ => continue,
            }
        }
    };
    terminal::disable_raw_mode().ok();
    println!("{}", if accepted { "y" } else { "N" });
    accepted
}

/// Resolves a project by name or numeric ID.
/// If the input is a name and no project is found, prompts the user to create it.
pub fn resolve_or_create_project(
    storage: &dyn Storage,
    user_id: &str,
    input: &str,
) -> Result<Project> {
    match ProjectService::new(storage, user_id).resolve(input) {
        Ok(p) => Ok(p),
        Err(TmkprError::NotFound {
            entity: "project", ..
        }) if input.parse::<u32>().is_err() => {
            if confirm(&format!("Project '{}' not found. Create it?", input)) {
                Ok(ProjectService::new(storage, user_id).add(input, None, None)?)
            } else {
                Err(TmkprError::NotFound {
                    entity: "project",
                    id: input.to_string(),
                }
                .into())
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Resolve optional --project and --task CLI args to model objects.
/// Returns `(None, None)` when both args are absent.
/// Errors if a task name is given without a project, or if resolution/creation fails.
pub fn resolve_project_and_task(
    storage: &dyn Storage,
    user_id: &str,
    project_arg: Option<&str>,
    task_arg: Option<&str>,
) -> Result<(Option<Project>, Option<Task>)> {
    let project = project_arg
        .map(|input| resolve_or_create_project(storage, user_id, input))
        .transpose()?;
    let task = match (task_arg, &project) {
        (Some(input), Some(proj)) => Some(resolve_or_create_task(storage, user_id, proj, input)?),
        (Some(name), None) => {
            return Err(
                TmkprError::Config(format!("task `{}` requires a project (use -p)", name)).into(),
            )
        }
        _ => None,
    };
    Ok((project, task))
}

/// Resolves a task by name or numeric ID within a project.
/// If the input is a name and no task is found, prompts the user to create it.
pub fn resolve_or_create_task(
    storage: &dyn Storage,
    user_id: &str,
    project: &Project,
    input: &str,
) -> Result<Task> {
    match TaskService::new(storage, user_id).resolve(&project.id, input) {
        Ok(t) => Ok(t),
        Err(TmkprError::NotFound { entity: "task", .. }) if input.parse::<u32>().is_err() => {
            if confirm(&format!(
                "Task '{}' not found in project '{}'. Create it?",
                input, project.name
            )) {
                Ok(TaskService::new(storage, user_id).add(&project.name, input, None)?)
            } else {
                Err(TmkprError::NotFound {
                    entity: "task",
                    id: input.to_string(),
                }
                .into())
            }
        }
        Err(e) => Err(e.into()),
    }
}
