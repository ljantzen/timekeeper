use anyhow::Result;
use chrono::Datelike;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::{EntryService, ProjectService};
use tmkpr_lib::storage::Storage;

use crate::cli::ReportArgs;
use crate::output::{self, ProjectIndex};

pub fn run(
    args: ReportArgs,
    storage: &dyn Storage,
    user_id: &str,
    time_fmt: TimeFormat,
    format: &str,
    color: bool,
) -> Result<()> {
    let svc = EntryService::new(storage, user_id);

    if args.week.is_some() || args.wweek {
        let week_str = args.week.as_deref().unwrap_or("current");
        let now = chrono::Local::now();
        let iso = now.iso_week();
        let year = args.year.unwrap_or_else(|| iso.year());
        let (year, week) = if week_str == "current" {
            (year, iso.week())
        } else {
            let w: u32 = week_str
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid week number: {}", week_str))?;
            (year, w)
        };
        let week_report = svc.week_report(year, week, args.wweek, args.tag)?;
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

    let report = svc.report(from, until, args.project.as_deref(), args.tag)?;

    let proj_svc = ProjectService::new(storage, user_id);
    let projects = proj_svc.list(false)?;
    let project_index = ProjectIndex(projects);

    output::print_report(&report, &project_index, format, color);
    Ok(())
}
