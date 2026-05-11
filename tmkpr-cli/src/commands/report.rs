use anyhow::Result;
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

    let svc = EntryService::new(storage, user_id);
    let report = svc.report(from, until, args.project.as_deref())?;

    output::print_report_table(&report, color);
    Ok(())
}
