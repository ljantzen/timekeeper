use std::ffi::OsStr;
use std::path::PathBuf;

use clap_complete::CompletionCandidate;
use tmkpr_lib::config::Config;
use tmkpr_lib::service::{ProjectService, TaskService};
use tmkpr_lib::storage::open_sqlite;

pub fn complete_projects(current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    list_project_names()
        .unwrap_or_default()
        .into_iter()
        .filter(|n| n.starts_with(prefix.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

pub fn complete_tasks(current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    let project = infer_project_from_completion_args();
    list_task_names(project.as_deref())
        .unwrap_or_default()
        .into_iter()
        .filter(|n| n.starts_with(prefix.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

fn open_db() -> anyhow::Result<(Box<dyn tmkpr_lib::storage::Storage>, String)> {
    let config = Config::load()?;
    let user_id = config.user.user_id.clone();
    let db_path = std::env::var_os("TMKPR_DB")
        .map(PathBuf::from)
        .unwrap_or(config.database.path);
    Ok((open_sqlite(&db_path)?, user_id))
}

fn list_project_names() -> anyhow::Result<Vec<String>> {
    let (storage, user_id) = open_db()?;
    let projects = ProjectService::new(storage.as_ref(), &user_id).list(false)?;
    Ok(projects.into_iter().map(|p| p.name).collect())
}

fn list_task_names(project: Option<&str>) -> anyhow::Result<Vec<String>> {
    let (storage, user_id) = open_db()?;
    let task_svc = TaskService::new(storage.as_ref(), &user_id);

    if let Some(proj) = project {
        let tasks = task_svc.list(proj, false).unwrap_or_default();
        return Ok(tasks.into_iter().map(|t| t.name).collect());
    }

    let proj_svc = ProjectService::new(storage.as_ref(), &user_id);
    let projects = proj_svc.list(false)?;
    let mut names: Vec<String> = projects
        .iter()
        .flat_map(|p| task_svc.list(&p.name, false).unwrap_or_default())
        .map(|t| t.name)
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

/// Extract --project / -p from the completion command line to scope task completions.
///
/// During dynamic completion the binary is invoked as:
///   tmkpr -- tmkpr <subcommand> [args …]
/// where everything after `--` is the full command line being completed.
fn infer_project_from_completion_args() -> Option<String> {
    let args: Vec<_> = std::env::args_os().collect();
    let cmd_start = args.iter().position(|a| a == "--").map(|i| i + 1)?;
    let cmd_args = &args[cmd_start..];

    for i in 0..cmd_args.len().saturating_sub(1) {
        if cmd_args[i] == "--project" || cmd_args[i] == "-p" {
            return cmd_args[i + 1].to_str().map(str::to_owned);
        }
        if let Some(s) = cmd_args[i].to_str() {
            if let Some(v) = s.strip_prefix("--project=") {
                return Some(v.to_owned());
            }
        }
    }
    None
}
