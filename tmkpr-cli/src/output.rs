use chrono::{DateTime, Local, Utc};
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, Color, Table};
use tmkpr_lib::service::WeekReport;

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

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&h[0..2], 16).ok()?,
        u8::from_str_radix(&h[2..4], 16).ok()?,
        u8::from_str_radix(&h[4..6], 16).ok()?,
    ))
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

    pub fn color(&self, id: &str) -> Option<(u8, u8, u8)> {
        self.0
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.color.as_deref())
            .and_then(hex_to_rgb)
    }

    pub fn color_by_name(&self, name: &str) -> Option<(u8, u8, u8)> {
        self.0
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.color.as_deref())
            .and_then(hex_to_rgb)
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
    table.load_preset(UTF8_FULL);
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

        let project_cell = {
            let cell = Cell::new(&project);
            if color {
                if let Some(pid) = entry.project_id.as_deref() {
                    if let Some((r, g, b)) = projects.color(pid) {
                        cell.fg(Color::Rgb { r, g, b })
                    } else {
                        cell
                    }
                } else {
                    cell
                }
            } else {
                cell
            }
        };

        let finished_cell = if color && entry.is_active() {
            Cell::new(finished).fg(Color::Green)
        } else {
            Cell::new(finished)
        };

        table.add_row(vec![
            id_cell,
            project_cell,
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

pub fn print_report_table(report: &ReportData, projects: &ProjectIndex, color: bool) {
    if report.by_project.is_empty() {
        println!("No entries in the selected range.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Project", "Task", "Entries", "Duration"]);

    for proj in &report.by_project {
        for (i, task) in proj.by_task.iter().enumerate() {
            let proj_cell = if i == 0 {
                let cell = Cell::new(&proj.project_name);
                if color {
                    if let Some((r, g, b)) = projects.color_by_name(&proj.project_name) {
                        cell.fg(Color::Rgb { r, g, b })
                    } else {
                        cell.fg(Color::Cyan)
                    }
                } else {
                    cell
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

pub fn print_projects_table(projects: &[Project], color: bool) {
    if projects.is_empty() {
        println!("No projects found.");
        return;
    }
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["#", "Name", "Description", "Color", "Archived"]);
    for p in projects {
        let name_cell = if color {
            if let Some((r, g, b)) = p.color.as_deref().and_then(hex_to_rgb) {
                Cell::new(&p.name).fg(Color::Rgb { r, g, b })
            } else {
                Cell::new(&p.name)
            }
        } else {
            Cell::new(&p.name)
        };
        table.add_row(vec![
            Cell::new(p.num_id).set_alignment(CellAlignment::Right),
            name_cell,
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
    table.load_preset(UTF8_FULL);
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
        let project_display = if let Some(pid) = entry.project_id.as_deref() {
            if let Some((r, g, b)) = projects.color(pid) {
                project.truecolor(r, g, b).to_string()
            } else {
                project.cyan().to_string()
            }
        } else {
            project.to_string()
        };
        print!("{}", project_display);
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

    let total_secs: i64 = gaps.iter().map(|(s, e)| (*e - *s).num_seconds()).sum();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
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

pub fn print_projects(projects: &[Project], format: &str, color: bool) {
    match format {
        "json" => print_json(projects),
        "csv" => print!("{}", projects_to_csv(projects)),
        _ => print_projects_table(projects, color),
    }
}

pub fn print_tasks(tasks: &[Task], format: &str) {
    match format {
        "json" => print_json(tasks),
        "csv" => print!("{}", tasks_to_csv(tasks)),
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
        "csv" => print!("{}", entries_to_csv(entries, projects, tasks, date_fmt)),
        "markdown" => print_entries_markdown(entries, projects, tasks, date_fmt),
        _ => print_entries_table(entries, projects, tasks, date_fmt, color),
    }
}

fn print_entries_markdown(
    entries: &[Entry],
    projects: &ProjectIndex,
    tasks: &TaskIndex,
    date_fmt: &str,
) {
    if entries.is_empty() {
        println!("No entries found.");
        return;
    }

    println!("| ID | Project | Task | Note | Tags | Started | Finished | Duration |");
    println!("|----|---------|----- |------|------|---------|----------|--------:|");

    for e in entries {
        let project = e
            .project_id
            .as_deref()
            .map(|id| projects.name(id))
            .unwrap_or_else(|| "-".to_string());
        let task = e
            .task_id
            .as_deref()
            .map(|id| tasks.name(id))
            .unwrap_or_else(|| "-".to_string());
        let note = e.note.as_deref().unwrap_or("-");
        let tags = if e.tags.is_empty() {
            "-".to_string()
        } else {
            e.tags.join(", ")
        };
        let started = format_datetime(&e.started_at, date_fmt);
        let finished = e
            .finished_at
            .as_ref()
            .map(|f| format_datetime(f, date_fmt))
            .unwrap_or_else(|| "active".to_string());
        let duration = format_duration(e.elapsed().num_seconds());
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            short_id(&e.id),
            project,
            task,
            note,
            tags,
            started,
            finished,
            duration,
        );
    }
}

pub fn print_report(report: &ReportData, projects: &ProjectIndex, format: &str, color: bool) {
    match format {
        "json" => print_json(report),
        "csv" => print!("{}", report_to_csv(report)),
        "markdown" => print_report_markdown(report),
        _ => print_report_table(report, projects, color),
    }
}

fn print_report_markdown(report: &ReportData) {
    if report.by_project.is_empty() {
        println!("No entries in the selected range.");
        return;
    }

    println!("| Project | Task | Entries | Duration |");
    println!("|---------|------|--------:|---------:|");

    for proj in &report.by_project {
        for (i, task) in proj.by_task.iter().enumerate() {
            let proj_cell = if i == 0 {
                proj.project_name.as_str()
            } else {
                ""
            };
            println!(
                "| {} | {} | {} | {} |",
                proj_cell,
                task.task_name,
                task.entry_count,
                format_duration(task.total_secs),
            );
        }
        println!(
            "| **{}** | | | **{}** |",
            proj.project_name,
            format_duration(proj.total_secs),
        );
    }

    println!(
        "| **TOTAL** | | | **{}** |",
        format_duration(report.total_secs),
    );
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
            table.load_preset(UTF8_FULL);
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

pub fn print_week_report(report: &WeekReport, format: &str) {
    if report.days.is_empty() || report.total_secs == 0 {
        println!("No entries for week {}-W{:02}.", report.year, report.week);
        return;
    }

    match format {
        "json" => {
            print_json(report);
            return;
        }
        "csv" => {
            let day_headers: Vec<String> = report
                .days
                .iter()
                .map(|d| d.date.format("%a %Y-%m-%d").to_string())
                .collect();
            println!("project,{},total", day_headers.join(","));
            for (proj, total) in &report.totals_by_project {
                let cols: Vec<String> = report
                    .days
                    .iter()
                    .map(|d| {
                        d.by_project
                            .iter()
                            .find(|(n, _)| n == proj)
                            .map(|(_, s)| s.to_string())
                            .unwrap_or_else(|| "0".to_string())
                    })
                    .collect();
                println!("{},{},{}", csv_escape(proj), cols.join(","), total);
            }
            let totals: Vec<String> = report
                .days
                .iter()
                .map(|d| d.total_secs.to_string())
                .collect();
            println!("TOTAL,{},{}", totals.join(","), report.total_secs);
            return;
        }
        "markdown" => {
            let day_headers: Vec<String> = report
                .days
                .iter()
                .map(|d| d.date.format("%a %m/%d").to_string())
                .collect();
            let sep: Vec<&str> = std::iter::repeat_n("---:", report.days.len() + 2).collect();
            println!("| Project | {} | Total |", day_headers.join(" | "));
            println!("| {} |", sep.join(" | "));
            for (proj, total) in &report.totals_by_project {
                let cols: Vec<String> = report
                    .days
                    .iter()
                    .map(|d| {
                        d.by_project
                            .iter()
                            .find(|(n, _)| n == proj)
                            .map(|(_, s)| format_duration(*s))
                            .unwrap_or_else(|| "-".to_string())
                    })
                    .collect();
                println!(
                    "| {} | {} | **{}** |",
                    proj,
                    cols.join(" | "),
                    format_duration(*total)
                );
            }
            let totals: Vec<String> = report
                .days
                .iter()
                .map(|d| format_duration(d.total_secs))
                .collect();
            println!(
                "| **TOTAL** | {} | **{}** |",
                totals.join(" | "),
                format_duration(report.total_secs)
            );
            return;
        }
        _ => {}
    }

    // table format
    let day_headers: Vec<String> = report
        .days
        .iter()
        .map(|d| d.date.format("%a\n%m/%d").to_string())
        .collect();

    let mut header = vec!["Project".to_string()];
    header.extend(day_headers);
    header.push("Total".to_string());
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(header);

    for (proj, total) in &report.totals_by_project {
        let mut row: Vec<Cell> = vec![Cell::new(proj)];
        for day in &report.days {
            let secs = day
                .by_project
                .iter()
                .find(|(n, _)| n == proj)
                .map(|(_, s)| *s)
                .unwrap_or(0);
            let cell = if secs > 0 {
                Cell::new(format_duration(secs)).set_alignment(CellAlignment::Right)
            } else {
                Cell::new("-").set_alignment(CellAlignment::Right)
            };
            row.push(cell);
        }
        row.push(Cell::new(format_duration(*total)).set_alignment(CellAlignment::Right));
        table.add_row(row);
    }

    // totals row
    let mut total_row: Vec<Cell> = vec![Cell::new("TOTAL")];
    for day in &report.days {
        total_row.push(
            Cell::new(if day.total_secs > 0 {
                format_duration(day.total_secs)
            } else {
                "-".to_string()
            })
            .set_alignment(CellAlignment::Right),
        );
    }
    total_row
        .push(Cell::new(format_duration(report.total_secs)).set_alignment(CellAlignment::Right));
    table.add_row(total_row);

    println!("Week {}-W{:02}", report.year, report.week);
    println!("{table}");
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn projects_to_csv(projects: &[Project]) -> String {
    let mut out = String::from("num_id,name,description,color,archived\n");
    for p in projects {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            p.num_id,
            csv_escape(&p.name),
            csv_escape(p.description.as_deref().unwrap_or("")),
            csv_escape(p.color.as_deref().unwrap_or("")),
            p.archived,
        ));
    }
    out
}

fn tasks_to_csv(tasks: &[Task]) -> String {
    let mut out = String::from("num_id,name,description,archived\n");
    for t in tasks {
        out.push_str(&format!(
            "{},{},{},{}\n",
            t.num_id,
            csv_escape(&t.name),
            csv_escape(t.description.as_deref().unwrap_or("")),
            t.archived,
        ));
    }
    out
}

fn entries_to_csv(
    entries: &[Entry],
    projects: &ProjectIndex,
    tasks: &TaskIndex,
    date_fmt: &str,
) -> String {
    let mut out = String::from("id,project,task,note,tags,started,finished,duration_secs\n");
    for e in entries {
        let project = e
            .project_id
            .as_deref()
            .map(|id| projects.name(id))
            .unwrap_or_else(|| "-".to_string());
        let task = e
            .task_id
            .as_deref()
            .map(|id| tasks.name(id))
            .unwrap_or_else(|| "-".to_string());
        let note = e.note.as_deref().unwrap_or("");
        let tags = e.tags.join(" ");
        let started = format_datetime(&e.started_at, date_fmt);
        let finished = e
            .finished_at
            .as_ref()
            .map(|f| format_datetime(f, date_fmt))
            .unwrap_or_else(|| "active".to_string());
        let secs = e.elapsed().num_seconds();
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            e.id,
            csv_escape(&project),
            csv_escape(&task),
            csv_escape(note),
            csv_escape(&tags),
            csv_escape(&started),
            csv_escape(&finished),
            secs,
        ));
    }
    out
}

fn report_to_csv(report: &ReportData) -> String {
    let mut out = String::from("project,task,entries,duration_secs\n");
    for proj in &report.by_project {
        for task in &proj.by_task {
            out.push_str(&format!(
                "{},{},{},{}\n",
                csv_escape(&proj.project_name),
                csv_escape(&task.task_name),
                task.entry_count,
                task.total_secs,
            ));
        }
    }
    out
}

fn print_json<T: serde::Serialize + ?Sized>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmkpr_lib::models::project::Project;
    use tmkpr_lib::models::task::Task;

    // ── format_duration ──────────────────────────────────────────────────────

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(1), "1s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(60), "1m 00s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3599), "59m 59s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3600), "1h 00m");
        assert_eq!(format_duration(3660), "1h 01m");
        assert_eq!(format_duration(7384), "2h 03m");
    }

    #[test]
    fn format_duration_negative_clamps_to_zero() {
        assert_eq!(format_duration(-1), "0s");
        assert_eq!(format_duration(-3600), "0s");
    }

    // ── csv_escape ───────────────────────────────────────────────────────────

    #[test]
    fn csv_escape_plain_string_unchanged() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape(""), "");
    }

    #[test]
    fn csv_escape_wraps_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_escape_wraps_newline() {
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn csv_escape_doubles_inner_quotes() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    // ── ProjectIndex / TaskIndex ─────────────────────────────────────────────

    fn make_project(id: &str, name: &str) -> Project {
        Project {
            id: id.to_string(),
            user_id: "u1".to_string(),
            num_id: 1,
            name: name.to_string(),
            description: None,
            color: None,
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_task(id: &str, name: &str) -> Task {
        Task {
            id: id.to_string(),
            user_id: "u1".to_string(),
            project_id: "p1".to_string(),
            num_id: 1,
            name: name.to_string(),
            description: None,
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn project_index_found() {
        let idx = ProjectIndex(vec![make_project("p1", "Alpha")]);
        assert_eq!(idx.name("p1"), "Alpha");
    }

    #[test]
    fn project_index_missing_falls_back_to_id() {
        let idx = ProjectIndex(vec![]);
        assert_eq!(idx.name("unknown-id"), "unknown-id");
    }

    #[test]
    fn task_index_found() {
        let idx = TaskIndex(vec![make_task("t1", "Backend")]);
        assert_eq!(idx.name("t1"), "Backend");
    }

    #[test]
    fn task_index_missing_falls_back_to_id() {
        let idx = TaskIndex(vec![]);
        assert_eq!(idx.name("missing"), "missing");
    }

    // ── short_id ─────────────────────────────────────────────────────────────

    #[test]
    fn short_id_truncates_long_ids() {
        assert_eq!(short_id("abcdefgh1234"), "abcdefgh");
    }

    #[test]
    fn short_id_leaves_short_ids_alone() {
        assert_eq!(short_id("abc"), "abc");
        assert_eq!(short_id("abcdefgh"), "abcdefgh");
    }

    // ── hex_to_rgb ───────────────────────────────────────────────────────────

    #[test]
    fn hex_to_rgb_valid_color() {
        assert_eq!(hex_to_rgb("#ff6600"), Some((255, 102, 0)));
        assert_eq!(hex_to_rgb("#000000"), Some((0, 0, 0)));
        assert_eq!(hex_to_rgb("#ffffff"), Some((255, 255, 255)));
        assert_eq!(hex_to_rgb("#FF6600"), Some((255, 102, 0)));
    }

    #[test]
    fn hex_to_rgb_invalid_color() {
        assert_eq!(hex_to_rgb("#fff"), None);
        assert_eq!(hex_to_rgb("#gggggg"), None);
        assert_eq!(hex_to_rgb("ff6600"), Some((255, 102, 0)));
        assert_eq!(hex_to_rgb(""), None);
    }

    #[test]
    fn project_index_color_found() {
        let p = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Project One".to_string(),
            description: None,
            color: Some("#ff6600".to_string()),
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let idx = ProjectIndex(vec![p]);
        assert_eq!(idx.color("p1"), Some((255, 102, 0)));
    }

    #[test]
    fn project_index_color_by_name() {
        let p = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Design".to_string(),
            description: None,
            color: Some("#ff0000".to_string()),
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let idx = ProjectIndex(vec![p]);
        assert_eq!(idx.color_by_name("Design"), Some((255, 0, 0)));
    }

    #[test]
    fn project_index_color_missing_returns_none() {
        let idx = ProjectIndex(vec![]);
        assert_eq!(idx.color("unknown"), None);
        assert_eq!(idx.color_by_name("Unknown"), None);
    }

    #[test]
    fn project_index_color_no_color_set_returns_none() {
        let p = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Project".to_string(),
            description: None,
            color: None,
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let idx = ProjectIndex(vec![p]);
        assert_eq!(idx.color("p1"), None);
    }

    // ── CSV format helpers ───────────────────────────────────────────────────

    #[test]
    fn projects_to_csv_empty_has_header_only() {
        let csv = projects_to_csv(&[]);
        assert_eq!(csv.trim(), "num_id,name,description,color,archived");
    }

    #[test]
    fn projects_to_csv_single_row() {
        let p = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Alpha".to_string(),
            description: Some("First project".to_string()),
            color: Some("#ff6600".to_string()),
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let csv = projects_to_csv(&[p]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert_eq!(lines[0], "num_id,name,description,color,archived");
        assert!(lines[1].starts_with("1,Alpha,"));
    }

    #[test]
    fn projects_to_csv_escapes_comma_in_name() {
        let p = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "My, Inc.".to_string(),
            description: None,
            color: None,
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let csv = projects_to_csv(&[p]);
        assert!(csv.contains("\"My, Inc.\""));
    }

    #[test]
    fn tasks_to_csv_empty_has_header_only() {
        let csv = tasks_to_csv(&[]);
        assert_eq!(csv.trim(), "num_id,name,description,archived");
    }

    #[test]
    fn tasks_to_csv_single_row() {
        let t = Task {
            id: "t1".to_string(),
            project_id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Backlog".to_string(),
            description: Some("Work queue".to_string()),
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            num_id: 1,
        };
        let csv = tasks_to_csv(&[t]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "num_id,name,description,archived");
        assert!(lines[1].starts_with("1,Backlog,"));
    }

    #[test]
    fn entries_to_csv_empty_has_header_only() {
        let csv = entries_to_csv(&[], &ProjectIndex(vec![]), &TaskIndex(vec![]), "%F");
        assert_eq!(
            csv.trim(),
            "id,project,task,note,tags,started,finished,duration_secs"
        );
    }

    #[test]
    fn entries_to_csv_resolves_project_and_task_names() {
        use tmkpr_lib::models::entry::Entry;
        let now = chrono::Utc::now();
        let e = Entry {
            id: "e1".to_string(),
            user_id: "u1".to_string(),
            project_id: Some("p1".to_string()),
            task_id: Some("t1".to_string()),
            note: Some("test note".to_string()),
            started_at: now,
            finished_at: Some(now),
            tags: vec!["tag1".to_string()],
            created_at: now,
            updated_at: now,
        };

        let proj = Project {
            id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Alpha".to_string(),
            description: None,
            color: None,
            archived: false,
            created_at: now,
            updated_at: now,
            num_id: 1,
        };
        let task = Task {
            id: "t1".to_string(),
            project_id: "p1".to_string(),
            user_id: "u1".to_string(),
            name: "Task One".to_string(),
            description: None,
            archived: false,
            created_at: now,
            updated_at: now,
            num_id: 1,
        };
        let csv = entries_to_csv(
            &[e],
            &ProjectIndex(vec![proj]),
            &TaskIndex(vec![task]),
            "%F",
        );

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert!(lines[1].contains("Alpha")); // project name resolved
        assert!(lines[1].contains("Task One")); // task name resolved
    }

    #[test]
    fn entries_to_csv_active_entry_shows_active() {
        use tmkpr_lib::models::entry::Entry;
        let now = chrono::Utc::now();
        let e = Entry {
            id: "e1".to_string(),
            user_id: "u1".to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now,
            finished_at: None, // active
            tags: vec![],
            created_at: now,
            updated_at: now,
        };

        let csv = entries_to_csv(&[e], &ProjectIndex(vec![]), &TaskIndex(vec![]), "%F");
        assert!(csv.contains("active"));
    }

    #[test]
    fn report_to_csv_empty_has_header_only() {
        let report = ReportData {
            from: None,
            until: None,
            total_secs: 0,
            by_project: vec![],
        };
        let csv = report_to_csv(&report);
        assert_eq!(csv.trim(), "project,task,entries,duration_secs");
    }

    #[test]
    fn report_to_csv_rows() {
        use tmkpr_lib::service::entry_service::{ProjectReport, TaskReport};

        let report = ReportData {
            from: None,
            until: None,
            total_secs: 3600,
            by_project: vec![ProjectReport {
                project_name: "Alpha".to_string(),
                total_secs: 3600,
                by_task: vec![TaskReport {
                    task_name: "Task1".to_string(),
                    total_secs: 3600,
                    entry_count: 1,
                }],
            }],
        };

        let csv = report_to_csv(&report);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert_eq!(lines[0], "project,task,entries,duration_secs");
        assert!(lines[1].contains("Alpha"));
        assert!(lines[1].contains("Task1"));
        assert!(lines[1].contains("1")); // entry_count
        assert!(lines[1].contains("3600")); // duration_secs
    }

    // ── Format dispatcher smoke tests ────────────────────────────────────────

    #[test]
    fn print_projects_json_does_not_panic() {
        print_projects(&[], "json", false);
    }

    #[test]
    fn print_projects_csv_does_not_panic() {
        print_projects(&[], "csv", false);
    }

    #[test]
    fn print_entries_markdown_does_not_panic() {
        print_entries(
            &[],
            &ProjectIndex(vec![]),
            &TaskIndex(vec![]),
            "%F",
            "markdown",
            false,
        );
    }

    #[test]
    fn print_entries_csv_does_not_panic() {
        print_entries(
            &[],
            &ProjectIndex(vec![]),
            &TaskIndex(vec![]),
            "%F",
            "csv",
            false,
        );
    }
}
