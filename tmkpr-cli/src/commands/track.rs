use anyhow::Result;
use chrono::Utc;
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
    let explicit_start = args.start.is_some();
    let started_at = match args.start.as_deref() {
        Some("continue" | "cont") => last_entry_end(storage, user_id)?,
        Some(s) => parse_datetime_now(s, time_fmt)?,
        None => Utc::now(),
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
        if started_at < active.started_at {
            return Err(anyhow::anyhow!(
                "start time {} is before active entry started at {}",
                output::format_datetime(&started_at, date_fmt),
                output::format_datetime(&active.started_at, date_fmt),
            ));
        }

        if !args.force {
            let projects =
                ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
            let tasks = active
                .project_id
                .as_ref()
                .and_then(|pid| storage.list_tasks(pid, false).ok())
                .unwrap_or_default();
            let proj = active
                .project_id
                .as_deref()
                .map(|id| projects.name(id))
                .unwrap_or_else(|| "-".to_string());
            let task_name = active
                .task_id
                .as_deref()
                .map(|id| TaskIndex(tasks).name(id))
                .unwrap_or_else(|| "-".to_string());

            let question = if explicit_start {
                format!(
                    "Stop '{}/{}' at {} and start new?",
                    proj,
                    task_name,
                    output::format_datetime(&started_at, date_fmt),
                )
            } else {
                format!(
                    "Currently tracking '{}/{}' for {}. Stop and start new?",
                    proj,
                    task_name,
                    output::format_duration(elapsed.num_seconds()),
                )
            };

            if !prompt::confirm(&question) {
                return Ok(());
            }
        }

        svc.stop(Some(started_at))?;
    }

    let entry = svc.start(
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        args.note,
        args.tags,
        Some(started_at),
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

pub(crate) fn last_entry_end(storage: &dyn Storage, user_id: &str) -> Result<chrono::DateTime<chrono::Utc>> {
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
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No previous entry found. Please provide an explicit start time."
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Timelike, Utc};
    use tmkpr_lib::models::entry::NewEntry;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::storage::sqlite::SqliteStorage;
    use tmkpr_lib::storage::Storage;

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn args(start: Option<&str>, force: bool) -> StartArgs {
        StartArgs {
            project: None,
            task: None,
            note: None,
            start: start.map(str::to_owned),
            tags: vec![],
            force,
        }
    }

    fn seed_active(storage: &dyn Storage, started_at: chrono::DateTime<Utc>) -> String {
        storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: None,
                started_at,
                finished_at: None,
                tags: vec![],
            })
            .unwrap()
            .id
    }

    #[test]
    fn no_active_entry_starts_normally() {
        let s = mem();
        run(args(None, false), &s, LOCAL_USER_ID, "%Y-%m-%d %H:%M", TimeFormat::H24, false).unwrap();
        assert!(EntryService::new(&s, LOCAL_USER_ID).status().unwrap().is_some());
    }

    #[test]
    fn force_handoff_stops_active_at_start_time() {
        let s = mem();
        let active_id = seed_active(&s, Utc::now() - Duration::hours(2));

        run(args(Some("1 hour ago"), true), &s, LOCAL_USER_ID, "%Y-%m-%d %H:%M", TimeFormat::H24, false).unwrap();

        let stopped = s.get_entry(&active_id).unwrap();
        assert!(!stopped.is_active(), "active entry was not stopped");

        // finished_at should be ~1 hour ago (allow 10s tolerance for test runner lag)
        let age_secs = (Utc::now() - stopped.finished_at.unwrap()).num_seconds();
        assert!(
            age_secs >= 3590 && age_secs <= 3610,
            "finished_at age was {}s, expected ~3600s",
            age_secs
        );

        // a new entry is now active
        assert!(EntryService::new(&s, LOCAL_USER_ID).status().unwrap().is_some());
    }

    #[test]
    fn start_before_active_errors() {
        let s = mem();
        seed_active(&s, Utc::now() - Duration::hours(1));

        let err = run(
            args(Some("2 hours ago"), true),
            &s,
            LOCAL_USER_ID,
            "%Y-%m-%d %H:%M",
            TimeFormat::H24,
            false,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("before active entry started"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn start_at_active_start_is_allowed() {
        // Zero-duration handoff: new task starts exactly when old one started.
        // Strip sub-seconds so the ISO string round-trips to the same instant.
        let s = mem();
        let t = (Utc::now() - Duration::hours(1)).with_nanosecond(0).unwrap();
        let active_id = seed_active(&s, t);
        let t_str = t.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        run(args(Some(&t_str), true), &s, LOCAL_USER_ID, "%Y-%m-%d %H:%M", TimeFormat::H24, false).unwrap();

        let stopped = s.get_entry(&active_id).unwrap();
        assert!(!stopped.is_active());
        assert_eq!(stopped.finished_at.unwrap(), t);
    }
}
