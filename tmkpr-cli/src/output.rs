use chrono::{DateTime, Local, Utc};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, Table};

use tmkpr_lib::models::{comment::Comment, entry::Entry, project::Project, task::Task};
use tmkpr_lib::service::entry_service::ReportData;

pub fn format_duration(secs: i64) -> String {
    if secs < 0 {
        return "0s".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

pub fn format_datetime(dt: &DateTime<Utc>, fmt: &str) -> String {
    dt.with_timezone(&Local).format(fmt).to_string()
}

fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

// ── Entry table ───────────────────────────────────────────────────────────────

pub struct ProjectIndex(pub Vec<Project>);
pub struct TaskIndex(pub Vec<Task>);

impl ProjectIndex {
    pub fn name(&self, id: &str) -> String {
        self.0
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

impl TaskIndex {
    pub fn name(&self, id: &str) -> String {
        self.0
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}

pub fn print_entries_table(
    entries: &[Entry],
    projects: &ProjectIndex,
    tasks: &TaskIndex,
    date_fmt: &str,
    color: bool,
) {
    if entries.is_empty() {
        println!("No entries found.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec![
        "ID", "Project", "Task", "Note", "Tags", "Started", "Finished", "Duration",
    ]);

    for entry in entries {
        let project = entry
            .project_id
            .as_deref()
            .map(|id| projects.name(id))
            .unwrap_or_else(|| "-".to_string());
        let task = entry
            .task_id
            .as_deref()
            .map(|id| tasks.name(id))
            .unwrap_or_else(|| "-".to_string());
        let note = entry.note.as_deref().unwrap_or("-");
        let tags = if entry.tags.is_empty() {
            "-".to_string()
        } else {
            entry.tags.join(", ")
        };
        let started = format_datetime(&entry.started_at, date_fmt);
        let finished = entry
            .finished_at
            .as_ref()
            .map(|f| format_datetime(f, date_fmt))
            .unwrap_or_else(|| "active".to_string());
        let duration = format_duration(entry.elapsed().num_seconds());

        let id_cell = if color && entry.is_active() {
            Cell::new(short_id(&entry.id)).fg(Color::Green)
        } else {
            Cell::new(short_id(&entry.id))
        };

        let finished_cell = if color && entry.is_active() {
            Cell::new(finished).fg(Color::Green)
        } else {
            Cell::new(finished)
        };

        table.add_row(vec![
            id_cell,
            Cell::new(project),
            Cell::new(task),
            Cell::new(note),
            Cell::new(tags),
            Cell::new(started),
            finished_cell,
            Cell::new(duration).set_alignment(CellAlignment::Right),
        ]);
    }

    println!("{table}");
}

// ── Report table ──────────────────────────────────────────────────────────────

pub fn print_report_table(report: &ReportData, color: bool) {
    if report.by_project.is_empty() {
        println!("No entries in the selected range.");
        return;
    }

    let mut table = Table::new();
    table.set_header(vec!["Project", "Task", "Entries", "Duration"]);

    for proj in &report.by_project {
        for (i, task) in proj.by_task.iter().enumerate() {
            let proj_cell = if i == 0 {
                let c = Cell::new(&proj.project_name);
                if color {
                    c.fg(Color::Cyan)
                } else {
                    c
                }
            } else {
                Cell::new("")
            };
            table.add_row(vec![
                proj_cell,
                Cell::new(&task.task_name),
                Cell::new(task.entry_count.to_string()).set_alignment(CellAlignment::Right),
                Cell::new(format_duration(task.total_secs)).set_alignment(CellAlignment::Right),
            ]);
        }

        let subtotal_label = if color {
            format!("{}", format!("  {} subtotal", proj.project_name).dimmed())
        } else {
            format!("  {} subtotal", proj.project_name)
        };
        table.add_row(vec![
            Cell::new(subtotal_label),
            Cell::new(""),
            Cell::new(""),
            Cell::new(format_duration(proj.total_secs)).set_alignment(CellAlignment::Right),
        ]);
    }

    let total_label = if color {
        format!("{}", "TOTAL".bold())
    } else {
        "TOTAL".to_string()
    };
    table.add_row(vec![
        Cell::new(total_label),
        Cell::new(""),
        Cell::new(""),
        Cell::new(format_duration(report.total_secs)).set_alignment(CellAlignment::Right),
    ]);

    println!("{table}");
}

// ── Project table ─────────────────────────────────────────────────────────────

pub fn print_projects_table(projects: &[Project]) {
    if projects.is_empty() {
        println!("No projects found.");
        return;
    }
    let mut table = Table::new();
    table.set_header(vec!["#", "Name", "Description", "Color", "Archived"]);
    for p in projects {
        table.add_row(vec![
            Cell::new(p.num_id).set_alignment(CellAlignment::Right),
            Cell::new(&p.name),
            Cell::new(p.description.as_deref().unwrap_or("-")),
            Cell::new(p.color.as_deref().unwrap_or("-")),
            Cell::new(if p.archived { "yes" } else { "no" }),
        ]);
    }
    println!("{table}");
}

// ── Task table ────────────────────────────────────────────────────────────────

pub fn print_tasks_table(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("No tasks found.");
        return;
    }
    let mut table = Table::new();
    table.set_header(vec!["#", "Name", "Description", "Archived"]);
    for t in tasks {
        table.add_row(vec![
            Cell::new(t.num_id).set_alignment(CellAlignment::Right),
            Cell::new(&t.name),
            Cell::new(t.description.as_deref().unwrap_or("-")),
            Cell::new(if t.archived { "yes" } else { "no" }),
        ]);
    }
    println!("{table}");
}

// ── Status ────────────────────────────────────────────────────────────────────

pub fn print_status(
    entry: &Entry,
    projects: &ProjectIndex,
    tasks: &TaskIndex,
    date_fmt: &str,
    color: bool,
) {
    let project = entry
        .project_id
        .as_deref()
        .map(|id| projects.name(id))
        .unwrap_or_else(|| "-".to_string());
    let task = entry
        .task_id
        .as_deref()
        .map(|id| tasks.name(id))
        .unwrap_or_else(|| "-".to_string());
    let note = entry.note.as_deref().unwrap_or("");
    let elapsed = format_duration(entry.elapsed().num_seconds());
    let started = format_datetime(&entry.started_at, date_fmt);

    if color {
        print!("{} ", "●".green().bold());
        print!("{}", project.cyan());
        if task != "-" {
            print!(" / {}", task.cyan());
        }
        print!("  {}", elapsed.yellow().bold());
        if !note.is_empty() {
            print!("  {}", note.dimmed());
        }
        println!("  (started {})", started.dimmed());
    } else {
        println!(
            "● {}{}  {}{}  (started {})",
            project,
            if task != "-" {
                format!(" / {}", task)
            } else {
                String::new()
            },
            elapsed,
            if note.is_empty() {
                String::new()
            } else {
                format!("  {}", note)
            },
            started,
        );
    }
}

// ── Gaps table ────────────────────────────────────────────────────────────────

pub fn print_gaps_table(
    gaps: &[(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)],
    date_fmt: &str,
    color: bool,
) {
    if gaps.is_empty() {
        println!("No gaps found.");
        return;
    }

    let total_secs: i64 = gaps
        .iter()
        .map(|(s, e)| (*e - *s).num_seconds())
        .sum();

    let mut table = Table::new();
    table.set_header(vec!["From", "To", "Duration"]);

    for (start, end) in gaps {
        let dur = (*end - *start).num_seconds();
        let from_cell = Cell::new(format_datetime(start, date_fmt));
        let to_cell = Cell::new(format_datetime(end, date_fmt));
        let dur_cell = if color {
            Cell::new(format_duration(dur)).fg(Color::Yellow)
        } else {
            Cell::new(format_duration(dur))
        };
        table.add_row(vec![
            from_cell,
            to_cell,
            dur_cell.set_alignment(CellAlignment::Right),
        ]);
    }

    println!("{table}");
    println!("Total untracked: {}", format_duration(total_secs));
}

// ── Format-dispatching helpers ────────────────────────────────────────────────

pub fn print_projects(projects: &[Project], format: &str) {
    match format {
        "json" => print_json(projects),
        "csv" => {
            println!("num_id,name,description,color,archived");
            for p in projects {
                println!(
                    "{},{},{},{},{}",
                    p.num_id,
                    csv_escape(&p.name),
                    csv_escape(p.description.as_deref().unwrap_or("")),
                    csv_escape(p.color.as_deref().unwrap_or("")),
                    p.archived,
                );
            }
        }
        _ => print_projects_table(projects),
    }
}

pub fn print_tasks(tasks: &[Task], format: &str) {
    match format {
        "json" => print_json(tasks),
        "csv" => {
            println!("num_id,name,description,archived");
            for t in tasks {
                println!(
                    "{},{},{},{}",
                    t.num_id,
                    csv_escape(&t.name),
                    csv_escape(t.description.as_deref().unwrap_or("")),
                    t.archived,
                );
            }
        }
        _ => print_tasks_table(tasks),
    }
}

pub fn print_entries(
    entries: &[Entry],
    projects: &ProjectIndex,
    tasks: &TaskIndex,
    date_fmt: &str,
    format: &str,
    color: bool,
) {
    match format {
        "json" => print_json(entries),
        "csv" => {
            println!("id,project,task,note,tags,started,finished,duration_secs");
            for e in entries {
                let project = e.project_id.as_deref().map(|id| projects.name(id)).unwrap_or_else(|| "-".to_string());
                let task = e.task_id.as_deref().map(|id| tasks.name(id)).unwrap_or_else(|| "-".to_string());
                let note = e.note.as_deref().unwrap_or("");
                let tags = e.tags.join(" ");
                let started = format_datetime(&e.started_at, date_fmt);
                let finished = e.finished_at.as_ref().map(|f| format_datetime(f, date_fmt)).unwrap_or_else(|| "active".to_string());
                let secs = e.elapsed().num_seconds();
                println!(
                    "{},{},{},{},{},{},{},{}",
                    e.id,
                    csv_escape(&project),
                    csv_escape(&task),
                    csv_escape(note),
                    csv_escape(&tags),
                    csv_escape(&started),
                    csv_escape(&finished),
                    secs,
                );
            }
        }
        _ => print_entries_table(entries, projects, tasks, date_fmt, color),
    }
}

pub fn print_report(report: &ReportData, format: &str, color: bool) {
    match format {
        "json" => print_json(report),
        "csv" => {
            println!("project,task,entries,duration_secs");
            for proj in &report.by_project {
                for task in &proj.by_task {
                    println!(
                        "{},{},{},{}",
                        csv_escape(&proj.project_name),
                        csv_escape(&task.task_name),
                        task.entry_count,
                        task.total_secs,
                    );
                }
            }
        }
        _ => print_report_table(report, color),
    }
}

pub fn print_json_entry(entry: &Entry) {
    print_json(entry);
}

// ── Comments table ────────────────────────────────────────────────────────────

pub fn print_comments(comments: &[Comment], date_fmt: &str, format: &str) {
    match format {
        "json" => print_json(comments),
        "csv" => {
            println!("id,entry_id,body,created_at");
            for c in comments {
                println!(
                    "{},{},{},{}",
                    c.id,
                    c.entry_id,
                    csv_escape(&c.body),
                    csv_escape(&format_datetime(&c.created_at, date_fmt)),
                );
            }
        }
        _ => {
            if comments.is_empty() {
                println!("No comments found.");
                return;
            }
            let mut table = Table::new();
            table.set_header(vec!["ID", "Entry", "Body", "Created"]);
            for c in comments {
                table.add_row(vec![
                    Cell::new(&c.id[..c.id.len().min(8)]),
                    Cell::new(&c.entry_id[..c.entry_id.len().min(8)]),
                    Cell::new(&c.body),
                    Cell::new(format_datetime(&c.created_at, date_fmt)),
                ]);
            }
            println!("{table}");
        }
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn print_json<T: serde::Serialize + ?Sized>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}
