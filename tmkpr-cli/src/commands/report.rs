use anyhow::Result;
use chrono::Datelike;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::ReportArgs;
use crate::output;

pub fn run(
    args: ReportArgs,
    storage: &dyn Storage,
    user_id: &str,
    time_fmt: TimeFormat,
    format: &str,
    color: bool,
) -> Result<()> {
    let svc = EntryService::new(storage, user_id);

    if let Some(week_str) = &args.week {
        let now = chrono::Local::now();
        let iso = now.iso_week();
        let (year, week) = if week_str == "current" {
            (iso.year(), iso.week())
        } else {
            let w: u32 = week_str
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid week number: {}", week_str))?;
            (iso.year(), w)
        };
        let week_report = svc.week_report(year, week)?;
        output::print_week_report(&week_report, format);
        return Ok(());
    }

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

    let report = svc.report(from, until, args.project.as_deref())?;

    output::print_report(&report, format, color);
    Ok(())
}
