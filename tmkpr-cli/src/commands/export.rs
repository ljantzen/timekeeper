use std::io::{self, Write};

use anyhow::Result;
use chrono::Local;
use tmkpr_lib::models::entry::EntryFilter;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::{EntryService, ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

use crate::cli::ExportArgs;
use crate::output::{ProjectIndex, TaskIndex};

pub fn run(args: ExportArgs, storage: &dyn Storage, user_id: &str, time_fmt: TimeFormat) -> Result<()> {
    let from = args
        .from
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;
    let until = args
        .until
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    let project_id = match args.project.as_deref() {
        Some(input) => Some(ProjectService::new(storage, user_id).resolve(input)?.id),
        None => None,
    };

    let task_id = match (args.task.as_deref(), &project_id) {
        (Some(input), Some(pid)) => {
            Some(TaskService::new(storage, user_id).resolve(pid, input)?.id)
        }
        _ => None,
    };

    let entries = EntryService::new(storage, user_id).list(EntryFilter {
        user_id: user_id.to_string(),
        project_id,
        task_id,
        from,
        until,
        tags: args.tag,
        include_active: !args.no_active,
        limit: None,
    })?;

    let all_projects = storage.list_projects(user_id, true).unwrap_or_default();
    let all_tasks: Vec<_> = all_projects
        .iter()
        .flat_map(|p| storage.list_tasks(&p.id, true).unwrap_or_default())
        .collect();
    let projects = ProjectIndex(all_projects);
    let tasks = TaskIndex(all_tasks);

    let csv = to_csv(&entries, &projects, &tasks);

    match &args.file {
        Some(path) => {
            std::fs::write(path, &csv)?;
            let word = if entries.len() == 1 { "entry" } else { "entries" };
            eprintln!("Exported {} {} to '{}'.", entries.len(), word, path.display());
        }
        None => {
            let stdout = io::stdout();
            stdout.lock().write_all(csv.as_bytes())?;
        }
    }

    Ok(())
}

fn escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn to_csv(
    entries: &[tmkpr_lib::models::entry::Entry],
    projects: &ProjectIndex,
    tasks: &TaskIndex,
) -> String {
    let mut out = String::from("project,task,start,end,note,tags\n");
    for e in entries {
        let project = e
            .project_id
            .as_deref()
            .map(|id| projects.name(id))
            .unwrap_or_default();
        let project = if project == "-" {
            String::new()
        } else {
            project
        };

        let task = e
            .task_id
            .as_deref()
            .map(|id| tasks.name(id))
            .unwrap_or_default();
        let task = if task == "-" { String::new() } else { task };

        let start = e
            .started_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let end = e
            .finished_at
            .map(|f| {
                f.with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_default();

        let note = e.note.as_deref().unwrap_or("");
        let tags = e.tags.join(",");

        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape(&project),
            escape(&task),
            escape(&start),
            escape(&end),
            escape(note),
            escape(&tags),
        ));
    }
    out
}
