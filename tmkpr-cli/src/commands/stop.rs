use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::StopArgs;
use crate::output;

pub fn run(
    args: StopArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    config: &Config,
) -> Result<()> {
    let finished_at = args
        .end
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let svc = EntryService::new(storage, user_id);
    let entry = svc.stop(finished_at)?;

    // Retrieve project and task names for logging
    let project_name = entry
        .project_id
        .as_ref()
        .and_then(|pid| storage.get_project(pid).ok())
        .map(|p| p.name);
    let task_name = entry
        .task_id
        .as_ref()
        .and_then(|tid| storage.get_task(tid).ok())
        .map(|t| t.name);

    // Log to Obsidian if enabled
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project_name.as_deref(),
        task_name.as_deref(),
        obsidian_logger::ActivityAction::Stopped,
    );

    let duration = output::format_duration(entry.duration().unwrap_or_default().num_seconds());
    let started = output::format_datetime(&entry.started_at, date_fmt);
    let finished = output::format_datetime(entry.finished_at.as_ref().unwrap(), date_fmt);

    println!(
        "Stopped tracking.  {}  ({} → {})",
        duration, started, finished
    );
    Ok(())
}
