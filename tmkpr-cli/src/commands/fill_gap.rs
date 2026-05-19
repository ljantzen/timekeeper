use anyhow::{anyhow, Result};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::FillGapArgs;

pub fn run(args: FillGapArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let svc = EntryService::new(storage, user_id);

    let id = match args.id {
        Some(id) => id,
        None => {
            let active = svc.status()?.ok_or_else(|| anyhow!("no active entry"))?;
            active.0.id
        }
    };

    let changed = svc.fill_gaps(&id)?;
    if changed {
        println!("Gaps filled.");
    } else {
        println!("No adjacent entries found.");
    }
    Ok(())
}
