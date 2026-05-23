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
            let word = if entries.len() == 1 {
                "entry"
            } else {
                "entries"
            };
            eprintln!(
                "Exported {} {} to '{}'.",
                entries.len(),
                word,
                path.display()
            );
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
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn resolved_name(id: Option<&str>, lookup: impl Fn(&str) -> String) -> String {
    match id {
        Some(id) => {
            let name = lookup(id);
            if name == "-" {
                String::new()
            } else {
                name
            }
        }
        None => String::new(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tmkpr_lib::models::entry::Entry;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::nlp::TimeFormat;
    use tmkpr_lib::storage::sqlite::SqliteStorage;

    use super::*;
    use crate::cli::ExportArgs;
    use crate::output::{ProjectIndex, TaskIndex};

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn make_entry(
        started_at: chrono::DateTime<Utc>,
        finished_at: Option<chrono::DateTime<Utc>>,
        note: Option<&str>,
        tags: Vec<&str>,
    ) -> Entry {
        Entry {
            id: "eid".into(),
            user_id: LOCAL_USER_ID.into(),
            project_id: None,
            task_id: None,
            note: note.map(Into::into),
            started_at,
            finished_at,
            tags: tags.into_iter().map(Into::into).collect(),
            created_at: started_at,
            updated_at: started_at,
        }
    }

    fn t(y: i32, mo: u32, d: u32, h: u32, m: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, m, 0).unwrap()
    }

    // ── escape ────────────────────────────────────────────────────────────────

    #[test]
    fn escape_plain_string_unchanged() {
        assert_eq!(escape("hello"), "hello");
    }

    #[test]
    fn escape_with_comma_wraps_in_quotes() {
        assert_eq!(escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn escape_with_quote_wraps_and_doubles_quote() {
        assert_eq!(escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn escape_with_newline_wraps_in_quotes() {
        assert_eq!(escape("line\nbreak"), "\"line\nbreak\"");
    }

    // ── to_csv ────────────────────────────────────────────────────────────────

    #[test]
    fn csv_empty_has_header_only() {
        let csv = to_csv(&[], &ProjectIndex(vec![]), &TaskIndex(vec![]));
        assert_eq!(csv.trim(), "project,task,start,end,note,tags");
    }

    #[test]
    fn csv_basic_row_has_correct_columns() {
        let entry = make_entry(
            t(2024, 1, 15, 9, 0),
            Some(t(2024, 1, 15, 10, 0)),
            Some("work"),
            vec![],
        );
        let csv = to_csv(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![]));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        let cols: Vec<&str> = lines[1].splitn(6, ',').collect();
        assert_eq!(cols[4], "work"); // note column
        assert_eq!(cols[5], ""); // tags empty
    }

    #[test]
    fn csv_active_entry_has_empty_end() {
        let entry = make_entry(t(2024, 1, 15, 9, 0), None, None, vec![]);
        let csv = to_csv(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![]));
        let row = csv.lines().nth(1).unwrap();
        // end column (index 3) should be empty: ",,"
        let cols: Vec<&str> = row.splitn(6, ',').collect();
        assert_eq!(cols[3], "");
    }

    #[test]
    fn csv_tags_joined_by_comma_and_escaped() {
        let entry = make_entry(
            t(2024, 1, 15, 9, 0),
            Some(t(2024, 1, 15, 10, 0)),
            None,
            vec!["dev", "ui"],
        );
        let csv = to_csv(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![]));
        let row = csv.lines().nth(1).unwrap();
        // tags column is last; comma-containing tags get quoted
        assert!(row.ends_with("\"dev,ui\""));
    }

    // ── to_json ───────────────────────────────────────────────────────────────

    #[test]
    fn json_basic_structure() {
        let entry = make_entry(
            t(2024, 1, 15, 9, 0),
            Some(t(2024, 1, 15, 10, 0)),
            Some("note"),
            vec![],
        );
        let json = to_json(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![])).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["note"], "note");
    }

    #[test]
    fn json_empty_fields_omitted() {
        let entry = make_entry(t(2024, 1, 15, 9, 0), None, None, vec![]);
        let json = to_json(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![])).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed[0].get("end").is_none());
        assert!(parsed[0].get("note").is_none());
        assert!(parsed[0].get("tags").is_none());
    }

    #[test]
    fn json_tags_as_array() {
        let entry = make_entry(
            t(2024, 1, 15, 9, 0),
            Some(t(2024, 1, 15, 10, 0)),
            None,
            vec!["dev", "ui"],
        );
        let json = to_json(&[entry], &ProjectIndex(vec![]), &TaskIndex(vec![])).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["tags"], serde_json::json!(["dev", "ui"]));
    }

    // ── format detection ──────────────────────────────────────────────────────

    #[test]
    fn json_extension_triggers_json_output() {
        let s = mem();
        let f = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        let args = ExportArgs {
            file: Some(f.path().to_path_buf()),
            project: None,
            task: None,
            from: None,
            until: None,
            tag: vec![],
            no_active: false,
        };
        run(args, &s, LOCAL_USER_ID, TimeFormat::H24, "table").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn format_flag_json_triggers_json_to_file() {
        let s = mem();
        let f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        let args = ExportArgs {
            file: Some(f.path().to_path_buf()),
            project: None,
            task: None,
            from: None,
            until: None,
            tag: vec![],
            no_active: false,
        };
        run(args, &s, LOCAL_USER_ID, TimeFormat::H24, "json").unwrap();
        let content = std::fs::read_to_string(f.path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_array());
    }
}
