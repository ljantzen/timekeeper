use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::MergeArgs;
use crate::output;

pub fn run(args: MergeArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let merged = if args.prev {
        svc.merge_into_prev(&args.id)?
    } else {
        svc.merge_into_next(&args.id)?
    };
    let short_first = output::short_id(&args.id);
    let short_merged = output::short_id(&merged.id);

    let (project_name, task_name) = output::entry_names(&merged, storage);
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &merged,
        project_name.as_deref(),
        task_name.as_deref(),
        obsidian_logger::ActivityAction::Merged,
    );

    println!("Merged entry {} into {}.", short_first, short_merged);
    Ok(())
}
