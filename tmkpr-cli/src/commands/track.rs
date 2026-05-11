use anyhow::Result;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::StartArgs;
use crate::output::{self, ProjectIndex, TaskIndex};
use crate::prompt;

pub fn run(
    args: StartArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
) -> Result<()> {
    let started_at = args
        .start
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let project = args
        .project
        .as_deref()
        .map(|input| prompt::resolve_or_create_project(storage, user_id, input))
        .transpose()?;

    let task = match (args.task.as_deref(), &project) {
        (Some(input), Some(proj)) => {
            Some(prompt::resolve_or_create_task(storage, user_id, proj, input)?)
        }
        (Some(name), None) => {
            return Err(tmkpr_lib::error::TmkprError::Config(format!(
                "task `{}` requires a project (use -p)",
                name
            ))
            .into())
        }
        _ => None,
    };

    let entry = EntryService::new(storage, user_id).start(
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        args.note,
        args.tags,
        started_at,
    )?;

    let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
    let tasks = entry
        .project_id
        .as_ref()
        .and_then(|pid| storage.list_tasks(pid, false).ok())
        .unwrap_or_default();

    println!("Started tracking.");
    output::print_status(&entry, &projects, &TaskIndex(tasks), date_fmt, color);
    Ok(())
}
