use anyhow::Result;
use tmkpr_lib::storage::Storage;

use crate::cli::TagListArgs;
use crate::output;

pub fn list(_args: TagListArgs, storage: &dyn Storage, user_id: &str, format: &str) -> Result<()> {
    let tags = storage.list_tags(user_id)?;
    output::print_tags(&tags, format);
    Ok(())
}
