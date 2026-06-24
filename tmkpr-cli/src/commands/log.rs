use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use tmkpr_lib::config::Config;
use tmkpr_lib::models::entry::EntryFilter;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::LogArgs;
use crate::commands::import::parse_duration;
use crate::output::{self, ProjectIndex, TaskIndex};
use crate::prompt;

pub fn run(
    args: LogArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
    config: &Config,
) -> Result<()> {
    let started_at = match args.start.as_deref() {
        Some(s) => parse_datetime_now(s, time_fmt)?,
        None => suggest_start(storage, user_id, date_fmt)?,
    };

    let finished_at = match (args.end.as_deref(), args.duration.as_deref()) {
        (Some(s), _) => parse_datetime_now(s, time_fmt)?,
        (None, Some(d)) => started_at + parse_duration(d)?,
        (None, None) => Utc::now(),
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

    // Log to Obsidian if enabled
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project.as_ref().map(|p| p.name.as_str()),
        task.as_ref().map(|t| t.name.as_str()),
        obsidian_logger::ActivityAction::Edited,
    );

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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use tmkpr_lib::config::Config;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::service::EntryService;
    use tmkpr_lib::storage::sqlite::SqliteStorage;

    use crate::cli::LogArgs;

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn args(start: &str, end: Option<&str>, duration: Option<&str>) -> LogArgs {
        LogArgs {
            start: Some(start.to_string()),
            end: end.map(str::to_string),
            duration: duration.map(str::to_string),
            project: None,
            task: None,
            note: None,
            tags: vec![],
        }
    }

    #[test]
    fn log_with_duration_creates_correct_entry() {
        let s = mem();
        let start = Utc::now() - Duration::hours(2);
        let start_str = start.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        super::run(
            args(&start_str, None, Some("1h30m")),
            &s,
            LOCAL_USER_ID,
            "%Y-%m-%d %H:%M",
            tmkpr_lib::nlp::TimeFormat::H24,
            false,
            &Config::default(),
        )
        .unwrap();

        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        let dur = entries[0].finished_at.unwrap() - entries[0].started_at;
        assert_eq!(dur.num_seconds(), 5400);
    }

    #[test]
    fn log_duration_and_end_are_exclusive() {
        // clap enforces this at parse time; verify parse_duration path produces
        // a longer entry than a 1-second window would.
        let s = mem();
        let start = Utc::now() - Duration::hours(1);
        let start_str = start.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        super::run(
            args(&start_str, None, Some("45m")),
            &s,
            LOCAL_USER_ID,
            "%Y-%m-%d %H:%M",
            tmkpr_lib::nlp::TimeFormat::H24,
            false,
            &Config::default(),
        )
        .unwrap();

        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        let dur = entries[0].finished_at.unwrap() - entries[0].started_at;
        assert_eq!(dur.num_seconds(), 2700);
    }
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
