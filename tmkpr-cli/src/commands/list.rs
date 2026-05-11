use anyhow::Result;
use tmkpr_lib::models::entry::EntryFilter;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::{EntryService, ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

use crate::cli::ListArgs;
use crate::output::{self, ProjectIndex, TaskIndex};

pub fn run(
    args: ListArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
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

    let task_id = match (args.task.as_deref(), &project_id) {
        (Some(input), Some(pid)) => Some(TaskService::new(storage, user_id).resolve(pid, input)?.id),
        _ => None,
    };

    let filter = EntryFilter {
        user_id: user_id.to_string(),
        project_id,
        task_id,
        from,
        until,
        tags: args.tag,
        limit: args.limit,
        include_active: args.active,
    };

    let entries = EntryService::new(storage, user_id).list(filter)?;

    let projects = ProjectIndex(storage.list_projects(user_id, true).unwrap_or_default());
    let all_tasks: Vec<_> = storage
        .list_projects(user_id, true)
        .unwrap_or_default()
        .iter()
        .flat_map(|p| storage.list_tasks(&p.id, true).unwrap_or_default())
        .collect();

    output::print_entries_table(&entries, &projects, &TaskIndex(all_tasks), date_fmt, color);
    Ok(())
}
