use anyhow::Result;
use chrono::Local;
use tmkpr_lib::models::entry::UpdateEntry;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::{EventAddArgs, EventDeleteArgs, EventEditArgs};
use crate::output::{self, ProjectIndex, TaskIndex};
use crate::prompt;

pub fn add(
    args: EventAddArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
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

    let projects = ProjectIndex(storage.list_projects(user_id, true).unwrap_or_default());
    let all_tasks: Vec<_> = storage
        .list_projects(user_id, true)
        .unwrap_or_default()
        .iter()
        .flat_map(|p| storage.list_tasks(&p.id, true).unwrap_or_default())
        .collect();

    println!("Logged event {}.", &entry.id[..entry.id.len().min(8)]);
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &TaskIndex(all_tasks),
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

    let projects = ProjectIndex(storage.list_projects(user_id, true).unwrap_or_default());
    let all_tasks: Vec<_> = storage
        .list_projects(user_id, true)
        .unwrap_or_default()
        .iter()
        .flat_map(|p| storage.list_tasks(&p.id, true).unwrap_or_default())
        .collect();

    println!("Updated event {}.", &entry.id[..entry.id.len().min(8)]);
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &TaskIndex(all_tasks),
        date_fmt,
        color,
    );
    Ok(())
}

pub fn delete(args: EventDeleteArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let entry = svc.get(&args.id)?;

    if !args.yes
        && !prompt::confirm(&format!(
            "Delete event {} (at {})?",
            &entry.id[..entry.id.len().min(8)],
            entry
                .started_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
        ))
    {
        println!("Cancelled.");
        return Ok(());
    }

    svc.delete(&args.id)?;
    println!("Deleted event {}.", &entry.id[..entry.id.len().min(8)]);
    Ok(())
}
