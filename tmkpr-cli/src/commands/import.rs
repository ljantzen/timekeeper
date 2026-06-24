use std::collections::HashMap;
use std::io::{self, Read};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use tmkpr_lib::models::entry::NewEntry;
use tmkpr_lib::models::project::NewProject;
use tmkpr_lib::models::task::NewTask;
use tmkpr_lib::nlp::parse_duration;
use tmkpr_lib::storage::Storage;

use crate::cli::ImportArgs;

// ── Datetime parsing (shared) ─────────────────────────────────────────────────

pub(super) fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim();
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S%z",
    ] {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Ok(dt.with_timezone(&Utc));
        }
    }
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M",
        "%m/%d/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M",
    ] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Local
                .from_local_datetime(&ndt)
                .single()
                .map(|dt| dt.with_timezone(&Utc))
                .ok_or_else(|| anyhow::anyhow!("ambiguous local time (DST transition): {s}"));
        }
    }
    for fmt in &["%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y"] {
        if let Ok(nd) = NaiveDate::parse_from_str(s, fmt) {
            if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
                return Local
                    .from_local_datetime(&ndt)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok_or_else(|| anyhow::anyhow!("ambiguous local time: {s}"));
            }
        }
    }
    bail!("unrecognised datetime: {:?}", s)
}

// ── Shared upsert + insert ────────────────────────────────────────────────────

// project_cache: name → (id, is_new_this_run)
// task_cache:    (project_id, task_name) → id
// Returns (projects_created, tasks_created).
#[allow(clippy::too_many_arguments)]
fn upsert_and_insert(
    project_name: &str,
    task_name: &str,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    note: Option<String>,
    tags: Vec<String>,
    storage: &dyn Storage,
    user_id: &str,
    project_cache: &mut HashMap<String, (String, bool)>,
    task_cache: &mut HashMap<(String, String), String>,
    dry_run: bool,
) -> Result<(u32, u32)> {
    if let Some(e) = end {
        if start >= e {
            bail!("start must be before end");
        }
    }

    // ── Project ───────────────────────────────────────────────────────────────
    let (project_id, projects_created, project_is_new) = if project_name.is_empty() {
        (None, 0u32, false)
    } else {
        match project_cache.get(project_name) {
            Some((id, is_new)) => (Some(id.clone()), 0, *is_new),
            None => {
                let (id, created, is_new) =
                    match storage.get_project_by_name(user_id, project_name)? {
                        Some(p) => (p.id, 0, false),
                        None => {
                            let new_id = if dry_run {
                                format!("__dry__{project_name}")
                            } else {
                                storage
                                    .create_project(NewProject {
                                        user_id: user_id.to_string(),
                                        name: project_name.to_string(),
                                        description: None,
                                        color: None,
                                    })?
                                    .id
                            };
                            (new_id, 1, true)
                        }
                    };
                project_cache.insert(project_name.to_string(), (id.clone(), is_new));
                (Some(id), created, is_new)
            }
        }
    };

    // ── Task ──────────────────────────────────────────────────────────────────
    let (task_id, tasks_created) = if task_name.is_empty() {
        (None, 0u32)
    } else if let Some(pid) = &project_id {
        let cache_key = (pid.clone(), task_name.to_string());
        match task_cache.get(&cache_key) {
            Some(id) => (Some(id.clone()), 0),
            None => {
                let (id, created) = if project_is_new {
                    // Project was just created — no tasks can pre-exist under it
                    let new_id = if dry_run {
                        format!("__dry_task__{task_name}")
                    } else {
                        storage
                            .create_task(NewTask {
                                user_id: user_id.to_string(),
                                project_id: pid.clone(),
                                name: task_name.to_string(),
                                description: None,
                            })?
                            .id
                    };
                    (new_id, 1)
                } else {
                    match storage.get_task_by_name(pid, task_name)? {
                        Some(t) => (t.id, 0),
                        None => {
                            let new_id = if dry_run {
                                format!("__dry_task__{task_name}")
                            } else {
                                storage
                                    .create_task(NewTask {
                                        user_id: user_id.to_string(),
                                        project_id: pid.clone(),
                                        name: task_name.to_string(),
                                        description: None,
                                    })?
                                    .id
                            };
                            (new_id, 1)
                        }
                    }
                };
                task_cache.insert(cache_key, id.clone());
                (Some(id), created)
            }
        }
    } else {
        bail!("task '{task_name}' requires a project");
    };

    // ── Entry ─────────────────────────────────────────────────────────────────
    if !dry_run {
        storage.create_entry(NewEntry {
            user_id: user_id.to_string(),
            project_id,
            task_id,
            note,
            started_at: start,
            finished_at: end,
            tags,
        })?;
    }

    Ok((projects_created, tasks_created))
}

// ── Shared summary printer ────────────────────────────────────────────────────

fn print_summary(entries: u32, projects: u32, tasks: u32, skipped: u32, dry_run: bool) {
    let word = if entries == 1 { "entry" } else { "entries" };
    if dry_run {
        println!(
            "[dry run] {entries} {word} to import: {projects} project(s) and {tasks} task(s) to create ({skipped} would be skipped)"
        );
    } else {
        println!(
            "Imported {entries} {word}: {projects} project(s) and {tasks} task(s) created ({skipped} skipped)."
        );
    }
}

// ── CSV path ──────────────────────────────────────────────────────────────────

struct Cols {
    project: Option<usize>,
    task: Option<usize>,
    start: Option<usize>,
    start_date: Option<usize>,
    start_time: Option<usize>,
    end: Option<usize>,
    end_date: Option<usize>,
    end_time: Option<usize>,
    duration: Option<usize>,
    note: Option<usize>,
    tags: Option<usize>,
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase().replace([' ', '-'], "_")
}

fn map_columns(headers: &csv::StringRecord) -> Result<Cols> {
    let mut cols = Cols {
        project: None,
        task: None,
        start: None,
        start_date: None,
        start_time: None,
        end: None,
        end_date: None,
        end_time: None,
        duration: None,
        note: None,
        tags: None,
    };
    for (i, h) in headers.iter().enumerate() {
        match normalize(h).as_str() {
            "project" | "project_name" | "client" => cols.project = Some(i),
            "task" | "task_name" | "activity" => cols.task = Some(i),
            "start" | "started_at" | "start_datetime" | "start_date_time" => cols.start = Some(i),
            "start_date" | "date" => cols.start_date = Some(i),
            "start_time" => cols.start_time = Some(i),
            "end" | "finish" | "finished_at" | "stop" | "end_datetime" => cols.end = Some(i),
            "end_date" => cols.end_date = Some(i),
            "end_time" => cols.end_time = Some(i),
            "duration" | "time" | "time_spent" => cols.duration = Some(i),
            "note" | "notes" | "description" | "comment" => cols.note = Some(i),
            "tags" | "tag" | "labels" | "label" => cols.tags = Some(i),
            _ => {}
        }
    }
    if cols.start.is_none() && cols.start_date.is_none() {
        let found: Vec<&str> = headers.iter().collect();
        bail!(
            "CSV must have a 'start' or 'start_date' column; found: {}",
            found.join(", ")
        );
    }
    Ok(cols)
}

fn parse_row_datetime(
    record: &csv::StringRecord,
    combined: Option<usize>,
    date_col: Option<usize>,
    time_col: Option<usize>,
) -> Result<Option<DateTime<Utc>>> {
    if let Some(i) = combined {
        let s = record.get(i).unwrap_or("").trim();
        return if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parse_datetime(s)?))
        };
    }
    if let Some(di) = date_col {
        let ds = record.get(di).unwrap_or("").trim();
        if ds.is_empty() {
            return Ok(None);
        }
        let ts = time_col.and_then(|ti| record.get(ti)).unwrap_or("").trim();
        let s = if ts.is_empty() {
            ds.to_string()
        } else {
            format!("{ds} {ts}")
        };
        return Ok(Some(parse_datetime(&s)?));
    }
    Ok(None)
}

fn process_csv_row(
    record: &csv::StringRecord,
    cols: &Cols,
    storage: &dyn Storage,
    user_id: &str,
    project_cache: &mut HashMap<String, (String, bool)>,
    task_cache: &mut HashMap<(String, String), String>,
    dry_run: bool,
) -> Result<(u32, u32)> {
    let start = parse_row_datetime(record, cols.start, cols.start_date, cols.start_time)
        .context("start")?
        .ok_or_else(|| anyhow::anyhow!("missing start time"))?;

    let end = {
        let from_col =
            parse_row_datetime(record, cols.end, cols.end_date, cols.end_time).context("end")?;
        if from_col.is_some() {
            from_col
        } else if let Some(di) = cols.duration {
            let ds = record.get(di).unwrap_or("").trim();
            if ds.is_empty() {
                None
            } else {
                let dur = parse_duration(ds).context("duration")?;
                if dur <= Duration::zero() {
                    bail!("duration must be positive");
                }
                Some(start + dur)
            }
        } else {
            None
        }
    };

    let project_name = cols
        .project
        .and_then(|i| record.get(i))
        .unwrap_or("")
        .trim();
    let task_name = cols.task.and_then(|i| record.get(i)).unwrap_or("").trim();
    let note = cols
        .note
        .and_then(|i| record.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let tags: Vec<String> = cols
        .tags
        .and_then(|i| record.get(i))
        .map(|s| {
            s.split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect()
        })
        .unwrap_or_default();

    upsert_and_insert(
        project_name,
        task_name,
        start,
        end,
        note,
        tags,
        storage,
        user_id,
        project_cache,
        task_cache,
        dry_run,
    )
}

fn is_stdin(args: &ImportArgs) -> bool {
    match &args.file {
        None => true,
        Some(p) => p.as_os_str() == "-",
    }
}

fn run_csv(args: &ImportArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let reader: Box<dyn Read> = if is_stdin(args) {
        Box::new(io::stdin())
    } else {
        let path = args.file.as_ref().unwrap();
        Box::new(
            std::fs::File::open(path)
                .with_context(|| format!("cannot open '{}'", path.display()))?,
        )
    };
    let mut rdr = csv::Reader::from_reader(reader);

    let headers = rdr.headers()?.clone();
    let cols = map_columns(&headers)?;

    let mut project_cache: HashMap<String, (String, bool)> = HashMap::new();
    let mut task_cache: HashMap<(String, String), String> = HashMap::new();
    let (mut entries, mut projects, mut tasks, mut skipped) = (0u32, 0u32, 0u32, 0u32);

    for (i, result) in rdr.records().enumerate() {
        let row_num = i + 2;
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                if args.skip_errors {
                    eprintln!("row {row_num}: {e:#}");
                    skipped += 1;
                    continue;
                }
                return Err(anyhow::Error::from(e)).context(format!("row {row_num}"));
            }
        };
        match process_csv_row(
            &record,
            &cols,
            storage,
            user_id,
            &mut project_cache,
            &mut task_cache,
            args.dry_run,
        ) {
            Ok((p, t)) => {
                entries += 1;
                projects += p;
                tasks += t;
            }
            Err(e) => {
                if args.skip_errors {
                    eprintln!("row {row_num}: {e:#}");
                    skipped += 1;
                } else {
                    return Err(e).context(format!("row {row_num}"));
                }
            }
        }
    }

    print_summary(entries, projects, tasks, skipped, args.dry_run);
    Ok(())
}

// ── JSON path ─────────────────────────────────────────────────────────────────

// Accepts tags as either a JSON array or a comma-separated string.
#[derive(Debug, Default)]
struct Tags(Vec<String>);

impl<'de> serde::Deserialize<'de> for Tags {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Tags;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or array of strings")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Tags, E> {
                Ok(Tags(
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                ))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Tags, A::Error> {
                let mut tags = Vec::new();
                while let Some(s) = seq.next_element::<String>()? {
                    let t = s.trim().to_string();
                    if !t.is_empty() {
                        tags.push(t);
                    }
                }
                Ok(Tags(tags))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(serde::Deserialize)]
struct JsonEntry {
    #[serde(default, alias = "project_name", alias = "client")]
    project: Option<String>,
    #[serde(default, alias = "task_name", alias = "activity")]
    task: Option<String>,
    #[serde(default, alias = "started_at", alias = "start_datetime")]
    start: Option<String>,
    #[serde(
        default,
        alias = "finished_at",
        alias = "end_datetime",
        alias = "finish",
        alias = "stop"
    )]
    end: Option<String>,
    #[serde(default, alias = "description", alias = "comment", alias = "notes")]
    note: Option<String>,
    #[serde(default)]
    tags: Tags,
    #[serde(default, alias = "time_spent", alias = "time")]
    duration: Option<String>,
}

fn process_json_entry(
    entry: &JsonEntry,
    storage: &dyn Storage,
    user_id: &str,
    project_cache: &mut HashMap<String, (String, bool)>,
    task_cache: &mut HashMap<(String, String), String>,
    dry_run: bool,
) -> Result<(u32, u32)> {
    let start_str = entry
        .start
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing 'start' field"))?;
    let start = parse_datetime(start_str).context("start")?;

    let end = match entry.end.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_datetime(s).context("end")?),
        None => match entry.duration.as_deref().filter(|s| !s.is_empty()) {
            Some(d) => {
                let dur = parse_duration(d).context("duration")?;
                if dur <= Duration::zero() {
                    bail!("duration must be positive");
                }
                Some(start + dur)
            }
            None => None,
        },
    };

    let project_name = entry.project.as_deref().unwrap_or("").trim();
    let task_name = entry.task.as_deref().unwrap_or("").trim();
    let note = entry
        .note
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let tags = entry.tags.0.clone();

    upsert_and_insert(
        project_name,
        task_name,
        start,
        end,
        note,
        tags,
        storage,
        user_id,
        project_cache,
        task_cache,
        dry_run,
    )
}

fn run_json(args: &ImportArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let content = if is_stdin(args) {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .context("reading JSON from stdin")?;
        s
    } else {
        let path = args.file.as_ref().unwrap();
        std::fs::read_to_string(path)
            .with_context(|| format!("cannot read '{}'", path.display()))?
    };
    let source = args
        .file
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<stdin>".into());
    let json_entries: Vec<JsonEntry> = serde_json::from_str(&content)
        .with_context(|| format!("cannot parse JSON from '{source}'"))?;

    let mut project_cache: HashMap<String, (String, bool)> = HashMap::new();
    let mut task_cache: HashMap<(String, String), String> = HashMap::new();
    let (mut entries, mut projects, mut tasks, mut skipped) = (0u32, 0u32, 0u32, 0u32);

    for (i, entry) in json_entries.iter().enumerate() {
        let idx = i + 1;
        match process_json_entry(
            entry,
            storage,
            user_id,
            &mut project_cache,
            &mut task_cache,
            args.dry_run,
        ) {
            Ok((p, t)) => {
                entries += 1;
                projects += p;
                tasks += t;
            }
            Err(e) => {
                if args.skip_errors {
                    eprintln!("entry {idx}: {e:#}");
                    skipped += 1;
                } else {
                    return Err(e).context(format!("entry {idx}"));
                }
            }
        }
    }

    print_summary(entries, projects, tasks, skipped, args.dry_run);
    Ok(())
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(args: ImportArgs, storage: &dyn Storage, user_id: &str, format: &str) -> Result<()> {
    let is_json = format == "json"
        || args
            .file
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

    if is_json {
        run_json(&args, storage, user_id)
    } else {
        run_csv(&args, storage, user_id)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::io::Write;

    use chrono::Utc;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::service::EntryService;
    use tmkpr_lib::storage::sqlite::SqliteStorage;
    use tmkpr_lib::storage::Storage;

    use super::*;
    use crate::cli::ImportArgs;

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    // Write content to a named temp file with the given extension and run import.
    fn csv_run(
        content: &str,
        skip_errors: bool,
        dry_run: bool,
        storage: &dyn Storage,
    ) -> Result<()> {
        let mut f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        let args = ImportArgs {
            file: Some(f.path().to_path_buf()),
            skip_errors,
            dry_run,
        };
        let result = run(args, storage, LOCAL_USER_ID, "csv");
        drop(f);
        result
    }

    fn json_run(content: &str, skip_errors: bool, storage: &dyn Storage) -> Result<()> {
        let mut f = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        let args = ImportArgs {
            file: Some(f.path().to_path_buf()),
            skip_errors,
            dry_run: false,
        };
        let result = run(args, storage, LOCAL_USER_ID, "csv"); // format auto-detected from .json ext
        drop(f);
        result
    }

    fn entry_count(storage: &dyn Storage) -> usize {
        EntryService::new(storage, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: true,
                ..Default::default()
            })
            .unwrap()
            .len()
    }

    // ── parse_datetime ────────────────────────────────────────────────────────

    #[test]
    fn datetime_rfc3339_with_tz() {
        let dt = parse_datetime("2024-01-15T09:00:00+00:00").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 15, 9, 0, 0).unwrap());
    }

    #[test]
    fn datetime_naive_ymd_hms() {
        // Just check it parses without error; exact value is timezone-dependent.
        parse_datetime("2024-01-15 09:00:00").unwrap();
    }

    #[test]
    fn datetime_naive_ymd_hm() {
        parse_datetime("2024-01-15 09:00").unwrap();
    }

    #[test]
    fn datetime_date_only_midnight() {
        parse_datetime("2024-01-15").unwrap();
    }

    #[test]
    fn datetime_european_dot_format() {
        parse_datetime("15.01.2024 09:00").unwrap();
    }

    #[test]
    fn datetime_invalid_errors() {
        assert!(parse_datetime("not a date").is_err());
        assert!(parse_datetime("").is_err());
    }

    // ── CSV imports ───────────────────────────────────────────────────────────

    #[test]
    fn csv_basic_entry_imported() {
        let s = mem();
        csv_run(
            "project,task,start,end,note,tags\n\
             Website,Frontend,2024-01-15 09:00,2024-01-15 10:30,Login page,\"dev,ui\"\n",
            false,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].note.as_deref(), Some("Login page"));
        assert_eq!(entries[0].tags, vec!["dev", "ui"]);
    }

    #[test]
    fn csv_creates_project_and_task() {
        let s = mem();
        csv_run(
            "project,task,start,end\n\
             NewProj,NewTask,2024-01-15 09:00,2024-01-15 10:00\n",
            false,
            false,
            &s,
        )
        .unwrap();
        assert!(s
            .get_project_by_name(LOCAL_USER_ID, "NewProj")
            .unwrap()
            .is_some());
        let proj = s
            .get_project_by_name(LOCAL_USER_ID, "NewProj")
            .unwrap()
            .unwrap();
        assert!(s.get_task_by_name(&proj.id, "NewTask").unwrap().is_some());
    }

    #[test]
    fn csv_reuses_existing_project_no_duplicate() {
        let s = mem();
        let csv = "project,task,start,end\n\
                   Proj,,2024-01-15 09:00,2024-01-15 10:00\n\
                   Proj,,2024-01-15 11:00,2024-01-15 12:00\n";
        csv_run(csv, false, false, &s).unwrap();
        assert_eq!(s.list_projects(LOCAL_USER_ID, false).unwrap().len(), 1);
        assert_eq!(entry_count(&s), 2);
    }

    #[test]
    fn csv_duration_column() {
        let s = mem();
        csv_run(
            "project,start,duration\n\
             Proj,2024-01-15 09:00,1:30:00\n",
            false,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        let dur = entries[0].finished_at.unwrap() - entries[0].started_at;
        assert_eq!(dur.num_seconds(), 5400);
    }

    #[test]
    fn csv_split_start_date_and_time() {
        let s = mem();
        csv_run(
            "start_date,start_time,end\n\
             2024-01-15,09:00,2024-01-15 10:00\n",
            false,
            false,
            &s,
        )
        .unwrap();
        assert_eq!(entry_count(&s), 1);
    }

    #[test]
    fn csv_active_entry_no_end() {
        let s = mem();
        csv_run(
            "start,note\n\
             2024-01-15 09:00,Running\n",
            false,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].finished_at.is_none());
    }

    #[test]
    fn csv_start_after_end_errors() {
        let s = mem();
        let err = csv_run(
            "start,end\n\
             2024-01-15 10:00,2024-01-15 09:00\n",
            false,
            false,
            &s,
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("start must be before end"),
            "{err:#}"
        );
        assert_eq!(entry_count(&s), 0);
    }

    #[test]
    fn csv_task_without_project_errors() {
        let s = mem();
        let err = csv_run(
            "task,start,end\n\
             Orphan,2024-01-15 09:00,2024-01-15 10:00\n",
            false,
            false,
            &s,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("project"), "{err:#}");
    }

    #[test]
    fn csv_skip_errors_continues_past_bad_rows() {
        let s = mem();
        csv_run(
            "start,end,note\n\
             2024-01-15 09:00,2024-01-15 10:00,Good 1\n\
             bad-date,2024-01-15 11:00,Bad row\n\
             2024-01-15 11:00,2024-01-15 12:00,Good 2\n",
            true,
            false,
            &s,
        )
        .unwrap();
        assert_eq!(entry_count(&s), 2);
    }

    #[test]
    fn csv_dry_run_does_not_write() {
        let s = mem();
        csv_run(
            "project,start,end\n\
             Proj,2024-01-15 09:00,2024-01-15 10:00\n",
            false,
            true,
            &s,
        )
        .unwrap();
        assert_eq!(entry_count(&s), 0);
        assert!(s.list_projects(LOCAL_USER_ID, false).unwrap().is_empty());
    }

    // ── JSON imports ──────────────────────────────────────────────────────────

    #[test]
    fn json_basic_entry_imported() {
        let s = mem();
        json_run(
            r#"[{"project":"P","task":"T","start":"2024-01-15 09:00","end":"2024-01-15 10:00","note":"hi"}]"#,
            false, &s,
        ).unwrap();
        assert_eq!(entry_count(&s), 1);
        assert!(s.get_project_by_name(LOCAL_USER_ID, "P").unwrap().is_some());
    }

    #[test]
    fn json_tags_as_array() {
        let s = mem();
        json_run(
            r#"[{"start":"2024-01-15 09:00","end":"2024-01-15 10:00","tags":["dev","ui"]}]"#,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries[0].tags, vec!["dev", "ui"]);
    }

    #[test]
    fn json_tags_as_comma_string() {
        let s = mem();
        json_run(
            r#"[{"start":"2024-01-15 09:00","end":"2024-01-15 10:00","tags":"dev,ui"}]"#,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries[0].tags, vec!["dev", "ui"]);
    }

    #[test]
    fn json_missing_start_errors() {
        let s = mem();
        let err = json_run(
            r#"[{"end":"2024-01-15 10:00","note":"no start"}]"#,
            false,
            &s,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("start"), "{err:#}");
    }

    // ── Gap 9: JSON duration field ────────────────────────────────────────────

    #[test]
    fn json_duration_field_creates_entry_with_correct_duration() {
        let s = mem();
        json_run(
            r#"[{"start":"2024-01-15 09:00","duration":"1:30:00"}]"#,
            false,
            &s,
        )
        .unwrap();
        let entries = EntryService::new(&s, LOCAL_USER_ID)
            .list(tmkpr_lib::models::entry::EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        let dur = entries[0].finished_at.unwrap() - entries[0].started_at;
        assert_eq!(dur.num_seconds(), 5400);
    }

    #[test]
    fn json_format_flag_selects_json_parser() {
        // Same content but file has .csv extension; --format json must select JSON path.
        let s = mem();
        let mut f = tempfile::Builder::new().suffix(".csv").tempfile().unwrap();
        f.write_all(
            br#"[{"start":"2024-01-15 09:00","end":"2024-01-15 10:00","note":"flag test"}]"#,
        )
        .unwrap();
        f.flush().unwrap();
        let args = ImportArgs {
            file: Some(f.path().to_path_buf()),
            skip_errors: false,
            dry_run: false,
        };
        run(args, &s, LOCAL_USER_ID, "json").unwrap();
        drop(f);
        assert_eq!(entry_count(&s), 1);
    }
}
