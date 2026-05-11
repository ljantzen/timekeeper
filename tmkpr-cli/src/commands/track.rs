use anyhow::Result;
use tmkpr_lib::models::entry::EntryFilter;
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
    let started_at = match args.start.as_deref() {
        Some("continue" | "cont") => Some(last_entry_end(storage, user_id)?),
        Some(s) => Some(parse_datetime_now(s, time_fmt)?),
        None => None,
    };

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

    let svc = EntryService::new(storage, user_id);

    if let Some((active, elapsed)) = svc.status()? {
        let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
        let tasks = active
            .project_id
            .as_ref()
            .and_then(|pid| storage.list_tasks(pid, false).ok())
            .unwrap_or_default();
        let proj = active.project_id.as_deref()
            .map(|id| projects.name(id))
            .unwrap_or_else(|| "-".to_string());
        let task_name = active.task_id.as_deref()
            .map(|id| TaskIndex(tasks).name(id))
            .unwrap_or_else(|| "-".to_string());
        let question = format!(
            "Currently tracking '{}/{}' for {}. Stop and start new?",
            proj,
            task_name,
            output::format_duration(elapsed.num_seconds()),
        );
        if !prompt::confirm(&question) {
            return Ok(());
        }
        svc.stop(None)?;
    }

    let entry = svc.start(
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

fn last_entry_end(storage: &dyn Storage, user_id: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    let entries = EntryService::new(storage, user_id).list(EntryFilter {
        user_id: user_id.to_string(),
        include_active: false,
        limit: Some(1),
        ..Default::default()
    })?;

    entries
        .into_iter()
        .next()
        .and_then(|e| e.finished_at)
        .ok_or_else(|| anyhow::anyhow!("No previous entry found. Please provide an explicit start time."))
}
