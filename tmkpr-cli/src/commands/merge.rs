use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::MergeArgs;

pub fn run(args: MergeArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let merged = svc.merge_into_next(&args.id)?;
    let short_first = &args.id[..args.id.len().min(8)];
    let short_merged = &merged.id[..merged.id.len().min(8)];

    // Retrieve project and task names for logging
    let project_name = merged
        .project_id
        .as_ref()
        .and_then(|pid| storage.get_project(pid).ok())
        .map(|p| p.name);
    let task_name = merged
        .task_id
        .as_ref()
        .and_then(|tid| storage.get_task(tid).ok())
        .map(|t| t.name);

    // Log to Obsidian if enabled
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
