use anyhow::Result;
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::output::{self, ProjectIndex, TaskIndex};

pub fn run(storage: &dyn Storage, user_id: &str, date_fmt: &str, format: &str, color: bool) -> Result<()> {
    let svc = EntryService::new(storage, user_id);
    match svc.status()? {
        None => {
            if format == "json" {
                println!("null");
            } else {
                println!("No active tracking session.");
            }
        }
        Some((entry, _elapsed)) => {
            let projects = ProjectIndex(storage.list_projects(user_id, false).unwrap_or_default());
            let tasks = entry
                .project_id
                .as_ref()
                .and_then(|pid| storage.list_tasks(pid, false).ok())
                .unwrap_or_default();
            if format == "json" {
                output::print_json_entry(&entry);
            } else {
                output::print_status(&entry, &projects, &TaskIndex(tasks), date_fmt, color);
            }
        }
    }
    Ok(())
}
