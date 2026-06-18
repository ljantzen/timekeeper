use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::DeleteArgs;

pub fn run(args: DeleteArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let entry = svc.get(&args.id)?;

    if !args.yes {
        eprint!(
            "Delete entry {} (started {})? [y/N] ",
            &entry.id[..entry.id.len().min(8)],
            entry.started_at.format("%Y-%m-%d %H:%M")
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Retrieve project and task names for logging before deletion
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
        obsidian_logger::ActivityAction::Deleted,
    );

    svc.delete(&args.id)?;
    println!("Deleted entry {}.", &entry.id[..entry.id.len().min(8)]);
    Ok(())
}
