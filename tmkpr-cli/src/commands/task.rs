use anyhow::Result;
use tmkpr_lib::models::task::UpdateTask;
use tmkpr_lib::service::{ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

use crate::cli::{TaskAddArgs, TaskDeleteArgs, TaskEditArgs, TaskListArgs};
use crate::output;
use crate::prompt;

pub fn add(args: TaskAddArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let project = prompt::resolve_or_create_project(storage, user_id, &args.project)?;
    let task =
        TaskService::new(storage, user_id).add(&project.name, args.name, args.description)?;
    println!(
        "Created task '{}' in project '{}'.",
        task.name, project.name
    );
    Ok(())
}

pub fn list(args: TaskListArgs, storage: &dyn Storage, user_id: &str, format: &str) -> Result<()> {
    let project = ProjectService::new(storage, user_id).resolve(&args.project)?;
    let tasks = TaskService::new(storage, user_id).list(&project.name, args.archived)?;
    output::print_tasks(&tasks, format);
    Ok(())
}

pub fn edit(args: TaskEditArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let project = prompt::resolve_or_create_project(storage, user_id, &args.project)?;

    let dest_project_id = args
        .move_to
        .as_deref()
        .map(|input| prompt::resolve_or_create_project(storage, user_id, input))
        .transpose()?
        .map(|p| p.id);

    let update = UpdateTask {
        name: args.name,
        description: match args.description.as_deref() {
            Some("") | Some("-") => Some(None),
            Some(s) => Some(Some(s.to_string())),
            None => None,
        },
        project_id: dest_project_id,
        archived: None,
    };
    let task = TaskService::new(storage, user_id).edit(&project.id, &args.task, update)?;
    println!("Updated task '{}'.", task.name);
    Ok(())
}

pub fn delete(args: TaskDeleteArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let project = ProjectService::new(storage, user_id).resolve(&args.project)?;
    TaskService::new(storage, user_id).delete(&project.name, &args.name, args.hard)?;
    if args.hard {
        println!("Deleted task '{}'.", args.name);
    } else {
        println!(
            "Archived task '{}'. Use --hard to permanently delete.",
            args.name
        );
    }
    Ok(())
}
