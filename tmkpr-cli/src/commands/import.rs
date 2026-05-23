use std::collections::HashMap;
use std::io::{self, Read};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use tmkpr_lib::models::entry::NewEntry;
use tmkpr_lib::models::project::NewProject;
use tmkpr_lib::models::task::NewTask;
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

// ── Duration parsing (shared) ─────────────────────────────────────────────────

pub(super) fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        bail!("empty duration");
    }
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() >= 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit() || c == '.')) {
        let h: i64 = parts[0].parse().context("hours in duration")?;
        let m: i64 = parts[1].parse().context("minutes in duration")?;
        let sec: i64 = if parts.len() == 3 {
            parts[2].split('.').next().unwrap_or("0").parse().context("seconds in duration")?
        } else {
            0
        };
        return Ok(Duration::seconds(h * 3600 + m * 60 + sec));
    }
    let lower = s.to_lowercase()
        .replace("hours", "h").replace("hour", "h")
        .replace("minutes", "m").replace("minute", "m")
        .replace("mins", "m").replace("min", "m")
        .replace("seconds", "s").replace("second", "s")
        .replace("secs", "s").replace("sec", "s");
    let mut total_secs: i64 = 0;
    let mut buf = String::new();
    let mut found_unit = false;
    for ch in lower.chars() {
        match ch {
            '0'..='9' | '.' => buf.push(ch),
            'h' => {
                let v: f64 = buf.trim().parse().context("hours value in duration")?;
                total_secs += (v * 3600.0) as i64;
                buf.clear();
                found_unit = true;
            }
            'm' => {
                let v: f64 = buf.trim().parse().context("minutes value in duration")?;
                total_secs += (v * 60.0) as i64;
                buf.clear();
                found_unit = true;
            }
            's' => {
                let v: f64 = buf.trim().parse().context("seconds value in duration")?;
                total_secs += v as i64;
                buf.clear();
                found_unit = true;
            }
            ' ' | '_' => {}
            _ => bail!("unexpected character '{ch}' in duration: {s:?}"),
        }
    }
    if found_unit {
        return Ok(Duration::seconds(total_secs));
    }
    if let Ok(secs) = s.parse::<f64>() {
        return Ok(Duration::seconds(secs as i64));
    }
    bail!("cannot parse duration: {s:?}")
}

// ── Shared upsert + insert ────────────────────────────────────────────────────

// project_cache: name → (id, is_new_this_run)
// task_cache:    (project_id, task_name) → id
// Returns (projects_created, tasks_created).
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
    project:    Option<usize>,
    task:       Option<usize>,
    start:      Option<usize>,
    start_date: Option<usize>,
    start_time: Option<usize>,
    end:        Option<usize>,
    end_date:   Option<usize>,
    end_time:   Option<usize>,
    duration:   Option<usize>,
    note:       Option<usize>,
    tags:       Option<usize>,
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase().replace([' ', '-'], "_")
}

fn map_columns(headers: &csv::StringRecord) -> Result<Cols> {
    let mut cols = Cols {
        project: None, task: None,
        start: None, start_date: None, start_time: None,
        end: None, end_date: None, end_time: None,
        duration: None, note: None, tags: None,
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
        return if s.is_empty() { Ok(None) } else { Ok(Some(parse_datetime(s)?)) };
    }
    if let Some(di) = date_col {
        let ds = record.get(di).unwrap_or("").trim();
        if ds.is_empty() {
            return Ok(None);
        }
        let ts = time_col.and_then(|ti| record.get(ti)).unwrap_or("").trim();
        let s = if ts.is_empty() { ds.to_string() } else { format!("{ds} {ts}") };
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
        let from_col = parse_row_datetime(record, cols.end, cols.end_date, cols.end_time)
            .context("end")?;
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

    let project_name = cols.project.and_then(|i| record.get(i)).unwrap_or("").trim();
    let task_name = cols.task.and_then(|i| record.get(i)).unwrap_or("").trim();
    let note = cols.note
        .and_then(|i| record.get(i))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let tags: Vec<String> = cols.tags
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
        project_name, task_name, start, end, note, tags,
        storage, user_id, project_cache, task_cache, dry_run,
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
            &record, &cols, storage, user_id,
            &mut project_cache, &mut task_cache, args.dry_run,
        ) {
            Ok((p, t)) => { entries += 1; projects += p; tasks += t; }
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
    #[serde(default, alias = "finished_at", alias = "end_datetime", alias = "finish", alias = "stop")]
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
    let start_str = entry.start.as_deref()
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
    let note = entry.note.as_deref().filter(|s| !s.is_empty()).map(|s| s.to_string());
    let tags = entry.tags.0.clone();

    upsert_and_insert(
        project_name, task_name, start, end, note, tags,
        storage, user_id, project_cache, task_cache, dry_run,
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
    let source = args.file.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<stdin>".into());
    let json_entries: Vec<JsonEntry> = serde_json::from_str(&content)
        .with_context(|| format!("cannot parse JSON from '{source}'"))?;

    let mut project_cache: HashMap<String, (String, bool)> = HashMap::new();
    let mut task_cache: HashMap<(String, String), String> = HashMap::new();
    let (mut entries, mut projects, mut tasks, mut skipped) = (0u32, 0u32, 0u32, 0u32);

    for (i, entry) in json_entries.iter().enumerate() {
        let idx = i + 1;
        match process_json_entry(
            entry, storage, user_id,
            &mut project_cache, &mut task_cache, args.dry_run,
        ) {
            Ok((p, t)) => { entries += 1; projects += p; tasks += t; }
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
        || args.file.as_ref()
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
