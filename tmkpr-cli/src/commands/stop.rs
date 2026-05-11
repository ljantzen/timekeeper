use anyhow::Result;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
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
) -> Result<()> {
    let finished_at = args
        .end
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let svc = EntryService::new(storage, user_id);
    let entry = svc.stop(finished_at)?;

    let duration = output::format_duration(entry.duration().unwrap_or_default().num_seconds());
    let started = output::format_datetime(&entry.started_at, date_fmt);
    let finished = output::format_datetime(entry.finished_at.as_ref().unwrap(), date_fmt);

    println!(
        "Stopped tracking.  {}  ({} → {})",
        duration, started, finished
    );
    Ok(())
}
