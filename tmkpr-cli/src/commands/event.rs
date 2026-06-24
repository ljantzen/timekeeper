use anyhow::Result;
use chrono::Local;
use tmkpr_lib::config::Config;
use tmkpr_lib::models::entry::{EntryFilter, UpdateEntry};
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::{EntryService, ProjectService};
use tmkpr_lib::storage::Storage;

use crate::cli::{EventAddArgs, EventDeleteArgs, EventEditArgs, EventListArgs};
use crate::output;
use crate::prompt;

pub fn list(
    args: EventListArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    format: &str,
    color: bool,
) -> Result<()> {
    let from = args
        .from
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;
    let until = args
        .until
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let project_id = match args.project.as_deref() {
        Some(input) => Some(ProjectService::new(storage, user_id).resolve(input)?.id),
        None => None,
    };

    let mut entries = EntryService::new(storage, user_id).list(EntryFilter {
        user_id: user_id.to_string(),
        project_id,
        task_id: None,
        from,
        until,
        tags: args.tag,
        limit: args.limit,
        include_active: false,
    })?;

    entries.retain(|e| e.is_event());

    if entries.is_empty() {
        match format {
            "json" => println!("[]"),
            "csv" => println!("id,project,task,note,tags,started,finished,duration_secs"),
            _ => println!("No events found."),
        }
        return Ok(());
    }

    let (projects, tasks) = output::build_indexes(storage, user_id);

    output::print_entries(
        &entries,
        &projects,
        &tasks,
        date_fmt,
        format,
        color,
    );
    Ok(())
}

pub fn add(
    args: EventAddArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
    config: &Config,
) -> Result<()> {
    let at = match args.at.as_deref() {
        Some(s) => parse_datetime_now(s, time_fmt)?,
        None => chrono::Utc::now(),
    };

    let project = args
        .project
        .as_deref()
        .map(|input| prompt::resolve_or_create_project(storage, user_id, input))
        .transpose()?;

    let task = match (args.task.as_deref(), &project) {
        (Some(input), Some(proj)) => Some(prompt::resolve_or_create_task(
            storage, user_id, proj, input,
        )?),
        (Some(name), None) => {
            return Err(anyhow::anyhow!(
                "task `{}` requires a project (use -p)",
                name
            ));
        }
        _ => None,
    };

    let entry = EntryService::new(storage, user_id).log_event(
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        args.note,
        args.tags,
        at,
    )?;

    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        obsidian_logger::ActivityAction::EventLogged,
    );

    let (projects, tasks) = output::build_indexes(storage, user_id);

    println!("Logged event {}.", output::short_id(&entry.id));
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &tasks,
        date_fmt,
        color,
    );
    Ok(())
}

pub fn edit(
    args: EventEditArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
    config: &Config,
) -> Result<()> {
    let at = args
        .at
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let (project_id, resolved_project) = match args.project.as_deref() {
        Some("") | Some("-") => (Some(None), None),
        Some(input) => {
            let p = prompt::resolve_or_create_project(storage, user_id, input)?;
            let id = p.id.clone();
            (Some(Some(id)), Some(p))
        }
        None => (None, None),
    };

    let task_id: Option<Option<String>> = match args.task.as_deref() {
        Some("") | Some("-") => Some(None),
        Some(input) => {
            let pid = match &project_id {
                Some(Some(id)) => Some(id.clone()),
                _ => {
                    EntryService::new(storage, user_id)
                        .get(&args.id)?
                        .project_id
                }
            };
            match pid {
                Some(pid) => {
                    let project = match &resolved_project {
                        Some(p) => p.clone(),
                        None => storage.get_project(&pid)?,
                    };
                    let task = prompt::resolve_or_create_task(storage, user_id, &project, input)?;
                    Some(Some(task.id))
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "task requires a project; set --project first"
                    ))
                }
            }
        }
        None => None,
    };

    let update = UpdateEntry {
        project_id,
        task_id,
        note: args.note.map(Some),
        started_at: at,
        finished_at: at.map(Some),
        tags: args.tags,
    };

    let svc = EntryService::new(storage, user_id);
    let entry = svc.update(&args.id, update)?;

    let (project_name, task_name) = output::entry_names(&entry, storage);
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project_name.as_deref(),
        task_name.as_deref(),
        obsidian_logger::ActivityAction::Edited,
    );

    let (projects, tasks) = output::build_indexes(storage, user_id);

    println!("Updated event {}.", output::short_id(&entry.id));
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &tasks,
        date_fmt,
        color,
    );
    Ok(())
}

pub fn delete(
    args: EventDeleteArgs,
    storage: &dyn Storage,
    user_id: &str,
    config: &Config,
) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let entry = svc.get(&args.id)?;

    if !args.yes
        && !prompt::confirm(&format!(
            "Delete event {} (at {})?",
            output::short_id(&entry.id),
            entry
                .started_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
        ))
    {
        println!("Cancelled.");
        return Ok(());
    }

    let (project_name, task_name) = output::entry_names(&entry, storage);
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project_name.as_deref(),
        task_name.as_deref(),
        obsidian_logger::ActivityAction::Deleted,
    );

    svc.delete(&args.id)?;
    println!("Deleted event {}.", output::short_id(&entry.id));
    Ok(())
}
