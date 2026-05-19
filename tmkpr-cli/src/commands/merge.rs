use anyhow::Result;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::MergeArgs;

pub fn run(args: MergeArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    let merged = svc.merge_into_next(&args.id)?;
    let short_first = &args.id[..args.id.len().min(8)];
    let short_merged = &merged.id[..merged.id.len().min(8)];
    println!("Merged entry {} into {}.", short_first, short_merged);
    Ok(())
}
