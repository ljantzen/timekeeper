use std::io::{self, Write};

use anyhow::Result;
use chrono::Local;
use tmkpr_lib::models::entry::Entry;
use tmkpr_lib::models::entry::EntryFilter;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::{EntryService, ProjectService, TaskService};
use tmkpr_lib::storage::Storage;

use crate::cli::ExportArgs;
use crate::output::{ProjectIndex, TaskIndex};

pub fn run(
    args: ExportArgs,
    storage: &dyn Storage,
    user_id: &str,
    time_fmt: TimeFormat,
    format: &str,
) -> Result<()> {
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

    // Determine format: explicit --format json flag, or .json file extension
    let use_json = format == "json"
        || args
            .file
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

    let content = if use_json {
        to_json(&entries, &projects, &tasks)?
    } else {
        to_csv(&entries, &projects, &tasks)
    };

    match &args.file {
        Some(path) => {
            std::fs::write(path, &content)?;
            let word = if entries.len() == 1 { "entry" } else { "entries" };
            eprintln!("Exported {} {} to '{}'.", entries.len(), word, path.display());
        }
        None => {
            let stdout = io::stdout();
            stdout.lock().write_all(content.as_bytes())?;
        }
    }

    Ok(())
}

// ── CSV ───────────────────────────────────────────────────────────────────────

fn escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn to_csv(entries: &[Entry], projects: &ProjectIndex, tasks: &TaskIndex) -> String {
    let mut out = String::from("project,task,start,end,note,tags\n");
    for e in entries {
        let project = resolved_name(e.project_id.as_deref(), |id| projects.name(id));
        let task = resolved_name(e.task_id.as_deref(), |id| tasks.name(id));
        let start = fmt_local(&e.started_at);
        let end = e.finished_at.as_ref().map(fmt_local).unwrap_or_default();
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

// ── JSON ──────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct JsonEntry {
    project: String,
    task: String,
    start: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    end: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    note: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

fn to_json(entries: &[Entry], projects: &ProjectIndex, tasks: &TaskIndex) -> Result<String> {
    let rows: Vec<JsonEntry> = entries
        .iter()
        .map(|e| JsonEntry {
            project: resolved_name(e.project_id.as_deref(), |id| projects.name(id)),
            task: resolved_name(e.task_id.as_deref(), |id| tasks.name(id)),
            start: fmt_local(&e.started_at),
            end: e.finished_at.as_ref().map(fmt_local).unwrap_or_default(),
            note: e.note.clone().unwrap_or_default(),
            tags: e.tags.clone(),
        })
        .collect();
    Ok(serde_json::to_string_pretty(&rows)? + "\n")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fmt_local(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
}

fn resolved_name(id: Option<&str>, lookup: impl Fn(&str) -> String) -> String {
    match id {
        Some(id) => {
            let name = lookup(id);
            if name == "-" { String::new() } else { name }
        }
        None => String::new(),
    }
}
