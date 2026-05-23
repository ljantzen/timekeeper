use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use tmkpr_lib::models::entry::NewEntry;
use tmkpr_lib::models::project::NewProject;
use tmkpr_lib::models::task::NewTask;
use tmkpr_lib::storage::Storage;

use crate::cli::ImportArgs;

// ── Column index mapping ──────────────────────────────────────────────────────

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

// ── Datetime parsing ──────────────────────────────────────────────────────────

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim();
    // Timezone-aware formats → convert to UTC directly
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S%z",
    ] {
        if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
            return Ok(dt.with_timezone(&Utc));
        }
    }
    // Naive datetime formats → interpret as local time → UTC
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
    // Date-only formats → local midnight → UTC
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
        let combined_str = if ts.is_empty() {
            ds.to_string()
        } else {
            format!("{ds} {ts}")
        };
        return Ok(Some(parse_datetime(&combined_str)?));
    }
    Ok(None)
}

// ── Duration parsing ──────────────────────────────────────────────────────────

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        bail!("empty duration");
    }

    // H:MM:SS or H:MM  (all segments must be digits or a dot for fractional seconds)
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() >= 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit() || c == '.')) {
        let h: i64 = parts[0].parse().context("hours in duration")?;
        let m: i64 = parts[1].parse().context("minutes in duration")?;
        let sec: i64 = if parts.len() == 3 {
            // drop fractional part
            parts[2].split('.').next().unwrap_or("0").parse().context("seconds in duration")?
        } else {
            0
        };
        return Ok(Duration::seconds(h * 3600 + m * 60 + sec));
    }

    // Natural: 1h30m, 1h, 30m, 30min, 1h 30min, 2h 15m 30s, …
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

    // Plain number → treat as seconds
    if let Ok(secs) = s.parse::<f64>() {
        return Ok(Duration::seconds(secs as i64));
    }

    bail!("cannot parse duration: {s:?}")
}

// ── Row processing ────────────────────────────────────────────────────────────

// project_cache: name → (id, is_new_this_run)
// task_cache:    (project_id, task_name) → id
fn process_row(
    record: &csv::StringRecord,
    cols: &Cols,
    storage: &dyn Storage,
    user_id: &str,
    project_cache: &mut HashMap<String, (String, bool)>,
    task_cache: &mut HashMap<(String, String), String>,
    dry_run: bool,
) -> Result<(u32, u32)> {
    // ── Start time ────────────────────────────────────────────────────────────
    let start = parse_row_datetime(record, cols.start, cols.start_date, cols.start_time)
        .context("start")?
        .ok_or_else(|| anyhow::anyhow!("missing start time"))?;

    // ── End time (from end column or duration) ────────────────────────────────
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

    if let Some(e) = end {
        if start >= e {
            bail!("start must be before end");
        }
    }

    // ── Project upsert ────────────────────────────────────────────────────────
    let project_name = cols.project.and_then(|i| record.get(i)).unwrap_or("").trim();
    let (project_id, projects_created, project_is_new) = if project_name.is_empty() {
        (None, 0u32, false)
    } else {
        match project_cache.get(project_name) {
            Some((id, is_new)) => (Some(id.clone()), 0, *is_new),
            None => {
                let (id, created, is_new) = match storage.get_project_by_name(user_id, project_name)? {
                    Some(p) => (p.id, 0, false),
                    None => {
                        let new_id = if dry_run {
                            format!("__dry__{project_name}")
                        } else {
                            storage.create_project(NewProject {
                                user_id: user_id.to_string(),
                                name: project_name.to_string(),
                                description: None,
                                color: None,
                            })?.id
                        };
                        (new_id, 1, true)
                    }
                };
                project_cache.insert(project_name.to_string(), (id.clone(), is_new));
                (Some(id), created, is_new)
            }
        }
    };

    // ── Task upsert ───────────────────────────────────────────────────────────
    let task_name = cols.task.and_then(|i| record.get(i)).unwrap_or("").trim();
    let (task_id, tasks_created) = if task_name.is_empty() {
        (None, 0u32)
    } else if let Some(pid) = &project_id {
        let cache_key = (pid.clone(), task_name.to_string());
        match task_cache.get(&cache_key) {
            Some(id) => (Some(id.clone()), 0),
            None => {
                // If the project is new this run, no existing tasks can exist under it
                let (id, created) = if project_is_new {
                    let new_id = if dry_run {
                        format!("__dry_task__{task_name}")
                    } else {
                        storage.create_task(NewTask {
                            user_id: user_id.to_string(),
                            project_id: pid.clone(),
                            name: task_name.to_string(),
                            description: None,
                        })?.id
                    };
                    (new_id, 1)
                } else {
                    match storage.get_task_by_name(pid, task_name)? {
                        Some(t) => (t.id, 0),
                        None => {
                            let new_id = if dry_run {
                                format!("__dry_task__{task_name}")
                            } else {
                                storage.create_task(NewTask {
                                    user_id: user_id.to_string(),
                                    project_id: pid.clone(),
                                    name: task_name.to_string(),
                                    description: None,
                                })?.id
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
        bail!("task '{task_name}' requires a project column");
    };

    // ── Note and tags ─────────────────────────────────────────────────────────
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

    // ── Write ─────────────────────────────────────────────────────────────────
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

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run(args: ImportArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let mut rdr = csv::Reader::from_path(&args.file)
        .with_context(|| format!("cannot open '{}'", args.file.display()))?;

    let headers = rdr.headers()?.clone();
    let cols = map_columns(&headers)?;

    let mut project_cache: HashMap<String, (String, bool)> = HashMap::new();
    let mut task_cache: HashMap<(String, String), String> = HashMap::new();

    let mut entries: u32 = 0;
    let mut projects: u32 = 0;
    let mut tasks: u32 = 0;
    let mut skipped: u32 = 0;

    for (i, result) in rdr.records().enumerate() {
        let row_num = i + 2; // row 1 is the header
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

        match process_row(
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

    let word = if entries == 1 { "entry" } else { "entries" };
    if args.dry_run {
        println!(
            "[dry run] {entries} {word} to import: {projects} project(s) and {tasks} task(s) to create ({skipped} row(s) would be skipped)"
        );
    } else {
        println!(
            "Imported {entries} {word}: {projects} project(s) and {tasks} task(s) created ({skipped} row(s) skipped)."
        );
    }

    Ok(())
}
