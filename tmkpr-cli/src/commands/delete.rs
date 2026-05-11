use anyhow::Result;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::DeleteArgs;

pub fn run(args: DeleteArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
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

    svc.delete(&args.id)?;
    println!("Deleted entry {}.", &entry.id[..entry.id.len().min(8)]);
    Ok(())
}
