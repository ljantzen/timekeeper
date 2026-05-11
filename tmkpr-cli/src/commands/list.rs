use anyhow::Result;
use chrono::{DateTime, Local, TimeZone, Utc};
use tmkpr_lib::models::entry::{Entry, EntryFilter};
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

    let task_id = match (args.task.as_deref(), &project_id) {
        (Some(input), Some(pid)) => Some(TaskService::new(storage, user_id).resolve(pid, input)?.id),
        _ => None,
    };

    if args.gaps {
        let explicit_from = from.is_some();
        let window_start = from.unwrap_or_else(|| {
            let today = Local::now().date_naive();
            Local
                .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap()
                .with_timezone(&Utc)
        });
        let window_end = until.unwrap_or_else(Utc::now);

        let entries = EntryService::new(storage, user_id).list(EntryFilter {
            user_id: user_id.to_string(),
            project_id,
            task_id,
            from: Some(window_start),
            until: Some(window_end),
            tags: args.tag,
            include_active: true,
            limit: None,
        })?;

        let mut gaps = compute_gaps(&entries, window_start, window_end);
        gaps.retain(|(s, e)| (*e - *s).num_seconds() > 0);
        if !explicit_from {
            if let Some(first) = gaps.first() {
                if first.0 == window_start {
                    gaps.remove(0);
                }
            }
        }
        output::print_gaps_table(&gaps, date_fmt, color);
        return Ok(());
    }

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

    output::print_entries(&entries, &projects, &TaskIndex(all_tasks), date_fmt, format, color);
    Ok(())
}

/// Returns the untracked gaps within [window_start, window_end].
fn compute_gaps(
    entries: &[Entry],
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let now = Utc::now();

    // Build sorted (start, end) intervals; active entries use now as their end.
    let mut intervals: Vec<(DateTime<Utc>, DateTime<Utc>)> = entries
        .iter()
        .map(|e| (e.started_at, e.finished_at.unwrap_or(now)))
        .collect();
    intervals.sort_by_key(|(s, _)| *s);

    // Merge overlapping / adjacent intervals.
    let mut merged: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
    for (start, end) in intervals {
        match merged.last_mut() {
            Some((_, prev_end)) if start <= *prev_end => {
                *prev_end = (*prev_end).max(end);
            }
            _ => merged.push((start, end)),
        }
    }

    // Find gaps between merged intervals and the window boundaries.
    let mut gaps = Vec::new();
    let mut cursor = window_start;

    for (start, end) in &merged {
        let start = (*start).max(window_start);
        if start > cursor {
            gaps.push((cursor, start));
        }
        cursor = cursor.max(*end);
    }

    if cursor < window_end {
        gaps.push((cursor, window_end));
    }

    gaps
}
