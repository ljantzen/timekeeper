use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use tmkpr_lib::models::entry::EntryFilter;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::LogArgs;
use crate::output::{self, ProjectIndex, TaskIndex};
use crate::prompt;

pub fn run(
    args: LogArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
) -> Result<()> {
    let started_at = match args.start.as_deref() {
        Some(s) => parse_datetime_now(s, time_fmt)?,
        None => suggest_start(storage, user_id, date_fmt)?,
    };

    let finished_at = args
        .end
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?
        .unwrap_or_else(Utc::now);

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

    let entry = EntryService::new(storage, user_id).log(
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        args.note,
        args.tags,
        started_at,
        finished_at,
    )?;

    let projects = ProjectIndex(storage.list_projects(user_id, true).unwrap_or_default());
    let all_tasks: Vec<_> = storage
        .list_projects(user_id, true)
        .unwrap_or_default()
        .iter()
        .flat_map(|p| storage.list_tasks(&p.id, true).unwrap_or_default())
        .collect();

    println!("Logged entry {}.", &entry.id[..entry.id.len().min(8)]);
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &TaskIndex(all_tasks),
        date_fmt,
        color,
    );
    Ok(())
}

fn suggest_start(
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
) -> Result<chrono::DateTime<Utc>> {
    let today_local = Local::now().date_naive();
    let today_start = Local
        .from_local_datetime(&today_local.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap()
        .with_timezone(&Utc);

    let entries = EntryService::new(storage, user_id).list(EntryFilter {
        user_id: user_id.to_string(),
        from: Some(today_start),
        include_active: false,
        ..Default::default()
    })?;

    let last_end = entries.iter().filter_map(|e| e.finished_at).max();

    match last_end {
        Some(t) => {
            let formatted = output::format_datetime(&t, date_fmt);
            if prompt::confirm(&format!("Use last entry end ({}) as start?", formatted)) {
                Ok(t)
            } else {
                Err(anyhow::anyhow!("Please provide --start"))
            }
        }
        None => Err(anyhow::anyhow!(
            "No previous entry found today. Please provide --start."
        )),
    }
}
