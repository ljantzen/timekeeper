use anyhow::Result;
use tmkpr_lib::config::Config;
use tmkpr_lib::obsidian_logger;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::DeleteArgs;
use crate::output;

pub fn run(args: DeleteArgs, storage: &dyn Storage, user_id: &str, config: &Config) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let entry = svc.get(&args.id)?;

    if !args.yes {
        eprint!(
            "Delete entry {} (started {})? [y/N] ",
            output::short_id(&entry.id),
            entry.started_at.format("%Y-%m-%d %H:%M")
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let (project_name, task_name) = output::entry_names(&entry, storage);
    let _ = obsidian_logger::log_activity_to_obsidian(
        config,
        &entry,
        project_name.as_deref(),
        task_name.as_deref(),
        obsidian_logger::ActivityAction::Deleted,
    );

    svc.delete(&args.id)?;
    println!("Deleted entry {}.", output::short_id(&entry.id));
    Ok(())
}
