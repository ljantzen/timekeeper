use std::io::{self, Write};

use anyhow::Result;
use tmkpr_lib::error::TmkprError;
use tmkpr_lib::models::{project::Project, task::Task};
use tmkpr_lib::service::{ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

pub fn confirm(question: &str) -> bool {
    print!("{} [y/N] ", question);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
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
        Err(TmkprError::ProjectNotFound(_)) if input.parse::<u32>().is_err() => {
            if confirm(&format!("Project '{}' not found. Create it?", input)) {
                Ok(ProjectService::new(storage, user_id).add(input, None, None)?)
            } else {
                Err(TmkprError::ProjectNotFound(input.to_string()).into())
            }
        }
        Err(e) => Err(e.into()),
    }
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
        Err(TmkprError::TaskNotFound(_)) if input.parse::<u32>().is_err() => {
            if confirm(&format!(
                "Task '{}' not found in project '{}'. Create it?",
                input, project.name
            )) {
                Ok(TaskService::new(storage, user_id).add(&project.name, input, None)?)
            } else {
                Err(TmkprError::TaskNotFound(input.to_string()).into())
            }
        }
        Err(e) => Err(e.into()),
    }
}
