use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::models::project::UpdateProject;
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::ProjectService;
use tmkpr_lib::storage::Storage;

use crate::cli::{ProjectAddArgs, ProjectDeleteArgs, ProjectEditArgs, ProjectListArgs};
use crate::output;

pub fn add(args: ProjectAddArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = ProjectService::new(storage, user_id);
    let project = svc.add(args.name, args.description, args.color)?;
    // Log to Obsidian if enabled
    let _ = obsidian_logger::log_project_created(config, &project);
    println!("Created project '{}'.", project.name);
    Ok(())
}

pub fn list(
    args: ProjectListArgs,
    storage: &dyn Storage,
    user_id: &str,
    format: &str,
    color: bool,
) -> Result<()> {
    let svc = ProjectService::new(storage, user_id);
    let projects = svc.list(args.archived)?;

    if format == "json" {
        let values: Vec<serde_json::Value> = projects
            .iter()
            .map(|p| {
                let mut v = serde_json::to_value(p).unwrap_or_default();
                let tasks = storage.list_tasks(&p.id, args.archived).unwrap_or_default();
                v["tasks"] = serde_json::to_value(&tasks).unwrap_or_default();
                v
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&values).unwrap_or_default()
        );
    } else {
        output::print_projects(&projects, format, color);
    }
    Ok(())
}

pub fn edit(args: ProjectEditArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let update = UpdateProject {
        name: args.name,
        description: match args.description.as_deref() {
            Some("") | Some("-") => Some(None),
            Some(s) => Some(Some(s.to_string())),
            None => None,
        },
        color: match args.color.as_deref() {
            Some("") | Some("-") => Some(None),
            Some(s) => Some(Some(s.to_string())),
            None => None,
        },
        archived: None,
    };
    let project = ProjectService::new(storage, user_id).edit(&args.project, update)?;
    // Log to Obsidian if enabled
    let _ = obsidian_logger::log_project_updated(config, &project);
    println!("Updated project '{}'.", project.name);
    Ok(())
}

pub fn delete(args: ProjectDeleteArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = ProjectService::new(storage, user_id);
    svc.delete(&args.name, args.hard)?;
    // Log to Obsidian if enabled
    let _ = obsidian_logger::log_project_deleted(config, &args.name);
    if args.hard {
        println!("Deleted project '{}'.", args.name);
    } else {
        println!(
            "Archived project '{}'. Use --hard to permanently delete.",
            args.name
        );
    }
    Ok(())
}
