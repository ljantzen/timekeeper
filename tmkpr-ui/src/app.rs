use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use tmkpr_lib::{
    models::{
        comment::Comment,
        entry::{parse_tags, Entry, EntryFilter, UpdateEntry},
        project::Project,
        task::Task,
    },
    nlp::parser::{parse_datetime, TimeFormat},
    service::{CommentService, EntryService, ProjectService, TaskService, WeekReport},
    storage::Storage,
};

use crate::form::{Field, Form};

fn parse_date_filter(s: &str) -> anyhow::Result<(Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>)> {
    let s = s.trim();
    if s.is_empty() {
        return Ok((None, None));
    }

    fn naive_to_utc(naive: chrono::NaiveDateTime) -> chrono::DateTime<Utc> {
        Local.from_local_datetime(&naive).unwrap().with_timezone(&Utc)
    }

    match s {
        "today" => {
            let now = Local::now();
            let from = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let until = (now.date_naive() + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();
            Ok((Some(naive_to_utc(from)), Some(naive_to_utc(until))))
        }
        "yesterday" => {
            let now = Local::now();
            let yesterday = now.date_naive() - Duration::days(1);
            let from = yesterday.and_hms_opt(0, 0, 0).unwrap();
            let until = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            Ok((Some(naive_to_utc(from)), Some(naive_to_utc(until))))
        }
        "this week" => {
            let now = Local::now();
            let weekday = now.date_naive().weekday();
            let days_since_monday = match weekday {
                chrono::Weekday::Mon => 0,
                chrono::Weekday::Tue => 1,
                chrono::Weekday::Wed => 2,
                chrono::Weekday::Thu => 3,
                chrono::Weekday::Fri => 4,
                chrono::Weekday::Sat => 5,
                chrono::Weekday::Sun => 6,
            };
            let week_start = now.date_naive() - Duration::days(days_since_monday as i64);
            let week_end = week_start + Duration::days(7);
            let from = week_start.and_hms_opt(0, 0, 0).unwrap();
            let until = week_end.and_hms_opt(0, 0, 0).unwrap();
            Ok((Some(naive_to_utc(from)), Some(naive_to_utc(until))))
        }
        _ if s.contains("..") => {
            let parts: Vec<&str> = s.split("..").collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid date range format"));
            }
            let from_date = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d")?;
            let until_date = NaiveDate::parse_from_str(parts[1].trim(), "%Y-%m-%d")?;
            let from = from_date.and_hms_opt(0, 0, 0).unwrap();
            let until = (until_date + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();
            Ok((Some(naive_to_utc(from)), Some(naive_to_utc(until))))
        }
        _ => {
            let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
            let from = date.and_hms_opt(0, 0, 0).unwrap();
            let until = (date + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap();
            Ok((Some(naive_to_utc(from)), Some(naive_to_utc(until))))
        }
    }
}

pub enum AppMode {
    Normal,
    StartModal(Form),
    EditModal {
        id: String,
        form: Form,
    },
    ConfirmDelete {
        id: String,
        display: String,
    },
    AddProject(Form),
    AddTask(Form),
    Filter(Form),
    Comments {
        entry_id: String,
        comments: Vec<Comment>,
        selected: usize,
    },
    AddComment {
        entry_id: String,
        form: Form,
    },
    Help,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ModeKind {
    Normal,
    StartModal,
    EditModal,
    ConfirmDelete,
    AddProject,
    AddTask,
    Filter,
    Comments,
    AddComment,
    Help,
}

impl AppMode {
    pub fn kind(&self) -> ModeKind {
        match self {
            AppMode::Normal => ModeKind::Normal,
            AppMode::StartModal(_) => ModeKind::StartModal,
            AppMode::EditModal { .. } => ModeKind::EditModal,
            AppMode::ConfirmDelete { .. } => ModeKind::ConfirmDelete,
            AppMode::AddProject(_) => ModeKind::AddProject,
            AppMode::AddTask(_) => ModeKind::AddTask,
            AppMode::Filter(_) => ModeKind::Filter,
            AppMode::Comments { .. } => ModeKind::Comments,
            AppMode::AddComment { .. } => ModeKind::AddComment,
            AppMode::Help => ModeKind::Help,
        }
    }
}

pub struct App {
    pub running: bool,
    pub storage: Box<dyn Storage>,
    pub user_id: String,
    pub entries: Vec<Entry>,
    pub active_entry: Option<Entry>,
    pub week_report: Option<WeekReport>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub selected: usize,
    pub list_state: ListState,
    pub mode: AppMode,
    pub status: Option<(String, bool)>,
    pub filter_project_name: String,
    pub filter_date_str: String,
    pub filter_project_id: Option<String>,
    pub filter_from: Option<chrono::DateTime<chrono::Utc>>,
    pub filter_until: Option<chrono::DateTime<chrono::Utc>>,
    pub entries_with_comments: HashSet<String>,
}

impl App {
    pub fn new(storage: Box<dyn Storage>, user_id: String) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            running: true,
            storage,
            user_id,
            entries: vec![],
            active_entry: None,
            week_report: None,
            projects: vec![],
            tasks: vec![],
            selected: 0,
            list_state,
            mode: AppMode::Normal,
            status: None,
            filter_project_name: String::new(),
            filter_date_str: String::new(),
            filter_project_id: None,
            filter_from: None,
            filter_until: None,
            entries_with_comments: HashSet::new(),
        }
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        {
            let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            self.projects = svc.list(false)?;
        }
        self.tasks = self
            .storage
            .list_all_tasks(&self.user_id, false)
            .unwrap_or_default();
        self.active_entry = self.storage.get_active_entry(&self.user_id)?;
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            self.entries = svc.list(EntryFilter {
                user_id: self.user_id.clone(),
                include_active: false,
                project_id: self.filter_project_id.clone(),
                from: self.filter_from,
                until: self.filter_until,
                ..Default::default()
            })?;
        }
        {
            let now = Local::now();
            let iso = now.iso_week();
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            self.week_report = svc.week_report(iso.year(), iso.week(), false).ok();
        }
        {
            let svc = CommentService::new(self.storage.as_ref(), &self.user_id);
            let all_comments = svc.list(None)?;
            self.entries_with_comments = all_comments.iter().map(|c| c.entry_id.clone()).collect();
        }
        if !self.entries.is_empty() && self.selected >= self.entries.len() {
            self.selected = self.entries.len() - 1;
        }
        self.list_state.select(Some(self.selected));
        Ok(())
    }

    fn project_names(&self) -> Vec<String> {
        self.projects.iter().map(|p| p.name.clone()).collect()
    }

    fn task_names(&self) -> Vec<String> {
        self.tasks.iter().map(|t| t.name.clone()).collect()
    }

    pub fn project_name<'a>(&'a self, id: &str) -> &'a str {
        self.projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.as_str())
            .unwrap_or("?")
    }

    pub fn task_name<'a>(&'a self, id: &str) -> &'a str {
        self.tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.as_str())
            .unwrap_or("?")
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn open_start_modal(&mut self) {
        let projects = self.project_names();
        let tasks = self.task_names();
        self.mode = AppMode::StartModal(Form {
            fields: vec![
                Field::new("Project", "").with_completions(projects),
                Field::new("Task", "").with_completions(tasks),
                Field::new("Note", ""),
                Field::new("Tags (comma-separated)", ""),
            ],
            focused: 0,
        });
    }

    pub fn open_edit_modal(&mut self) {
        let entry = &self.entries[self.selected];
        let id = entry.id.clone();
        let project_val = entry
            .project_id
            .as_deref()
            .map(|pid| self.project_name(pid).to_string())
            .unwrap_or_default();
        let task_val = entry
            .task_id
            .as_deref()
            .map(|tid| self.task_name(tid).to_string())
            .unwrap_or_default();
        let note_val = entry.note.clone().unwrap_or_default();
        let start_val = entry
            .started_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let end_val = entry
            .finished_at
            .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        let tags_val = entry.tags.join(", ");
        let projects = self.project_names();
        let tasks = self.task_names();
        self.mode = AppMode::EditModal {
            id,
            form: Form {
                fields: vec![
                    Field::new("Project", project_val).with_completions(projects),
                    Field::new("Task", task_val).with_completions(tasks),
                    Field::new("Note", note_val),
                    Field::new("Start", start_val),
                    Field::new("End (blank = active)", end_val),
                    Field::new("Tags (comma-separated)", tags_val),
                ],
                focused: 0,
            },
        };
    }

    pub fn open_confirm_delete(&mut self) {
        let entry = &self.entries[self.selected];
        let short_id = &entry.id[..8.min(entry.id.len())];
        let display = format!("entry {short_id}");
        self.mode = AppMode::ConfirmDelete {
            id: entry.id.clone(),
            display,
        };
    }

    pub fn stop_active(&mut self) -> anyhow::Result<()> {
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.stop(None)?;
        }
        self.refresh()?;
        self.status = Some(("Stopped.".into(), false));
        Ok(())
    }

    pub fn delete_entry(&mut self, id: &str) -> anyhow::Result<()> {
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.delete(id)?;
        }
        self.refresh()?;
        self.status = Some(("Entry deleted.".into(), false));
        Ok(())
    }

    pub fn start_entry(
        &mut self,
        project: &str,
        task: &str,
        note: &str,
        tags_str: &str,
    ) -> anyhow::Result<()> {
        let project_opt = if project.is_empty() {
            None
        } else {
            Some(project)
        };
        let task_opt = if task.is_empty() { None } else { Some(task) };
        let note_opt = if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        };
        let tags = parse_tags(tags_str);
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.start(project_opt, task_opt, note_opt, tags, None)?;
        }
        self.refresh()?;
        self.status = Some(("Started.".into(), false));
        Ok(())
    }

    pub fn edit_entry(
        &mut self,
        id: &str,
        project: &str,
        task: &str,
        note: &str,
        start_str: &str,
        end_str: &str,
        tags_str: &str,
    ) -> anyhow::Result<()> {
        let now = Utc::now();

        let started_at = if start_str.is_empty() {
            None
        } else {
            Some(
                parse_datetime(start_str, now, TimeFormat::H24)
                    .map_err(|e| anyhow::anyhow!("Invalid start time: {}", e))?,
            )
        };

        let finished_at = if end_str.is_empty() {
            // Leave finished_at unchanged — None here means "don't update the field"
            None
        } else {
            let parsed = parse_datetime(end_str, now, TimeFormat::H24)
                .map_err(|e| anyhow::anyhow!("Invalid end time: {}", e))?;
            Some(Some(parsed))
        };

        let project_id_update = if project.is_empty() {
            Some(None) // clear project
        } else {
            let proj = {
                let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
                svc.resolve(project)?
            };
            Some(Some(proj.id))
        };

        let task_id_update = if task.is_empty() {
            Some(None) // clear task
        } else {
            match &project_id_update {
                Some(Some(pid)) => {
                    let pid = pid.clone();
                    let task_obj = {
                        let svc = TaskService::new(self.storage.as_ref(), &self.user_id);
                        svc.resolve(&pid, task)?
                    };
                    Some(Some(task_obj.id))
                }
                _ => return Err(anyhow::anyhow!("Task requires a project")),
            }
        };

        let note_update = Some(if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        });

        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.update(
                id,
                UpdateEntry {
                    project_id: project_id_update,
                    task_id: task_id_update,
                    note: note_update,
                    started_at,
                    finished_at,
                    tags: Some(parse_tags(tags_str)),
                },
            )?;
        }
        self.refresh()?;
        self.status = Some(("Updated.".into(), false));
        Ok(())
    }

    pub fn open_comments(&mut self) -> anyhow::Result<()> {
        let entry_id = self.entries[self.selected].id.clone();
        self.refresh_comments_mode(entry_id, 0)
    }

    pub fn entry_display(&self, entry_id: &str) -> String {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == entry_id)
            .or_else(|| self.active_entry.as_ref().filter(|e| e.id == entry_id));
        match entry {
            Some(e) => match (&e.project_id, &e.task_id) {
                (Some(pid), Some(tid)) => {
                    format!("{} / {}", self.project_name(pid), self.task_name(tid))
                }
                (Some(pid), None) => self.project_name(pid).to_string(),
                _ => e
                    .note
                    .clone()
                    .unwrap_or_else(|| entry_id[..8.min(entry_id.len())].to_string()),
            },
            None => entry_id[..8.min(entry_id.len())].to_string(),
        }
    }

    pub fn open_comments_for_active(&mut self) -> anyhow::Result<()> {
        if let Some(entry) = &self.active_entry {
            let entry_id = entry.id.clone();
            self.refresh_comments_mode(entry_id, 0)?;
        } else {
            self.status = Some(("No active entry.".into(), true));
        }
        Ok(())
    }

    pub fn open_add_comment(&mut self) {
        if let AppMode::Comments { entry_id, .. } = &self.mode {
            let entry_id = entry_id.clone();
            self.mode = AppMode::AddComment {
                entry_id,
                form: Form {
                    fields: vec![Field::new("Comment", "")],
                    focused: 0,
                },
            };
        }
    }

    pub fn submit_add_comment(&mut self, entry_id: String, body: String) -> anyhow::Result<()> {
        if body.is_empty() {
            return Err(anyhow::anyhow!("Comment cannot be empty"));
        }
        {
            let svc = CommentService::new(self.storage.as_ref(), &self.user_id);
            svc.add(Some(&entry_id), body)?;
        }
        self.refresh_comments_mode(entry_id, 0)?;
        self.status = Some(("Comment added.".into(), false));
        Ok(())
    }

    pub fn cancel_add_comment(&mut self) -> anyhow::Result<()> {
        if let AppMode::AddComment { entry_id, .. } = &self.mode {
            let entry_id = entry_id.clone();
            self.refresh_comments_mode(entry_id, 0)?;
        }
        Ok(())
    }

    pub fn delete_selected_comment(&mut self) -> anyhow::Result<()> {
        let (comment_id, entry_id) = if let AppMode::Comments {
            comments,
            selected,
            entry_id,
        } = &self.mode
        {
            if comments.is_empty() {
                return Ok(());
            }
            (comments[*selected].id.clone(), entry_id.clone())
        } else {
            return Ok(());
        };
        {
            let svc = CommentService::new(self.storage.as_ref(), &self.user_id);
            svc.delete(&comment_id)?;
        }
        self.refresh_comments_mode(entry_id, 0)?;
        self.status = Some(("Comment deleted.".into(), false));
        Ok(())
    }

    fn refresh_comments_mode(&mut self, entry_id: String, selected: usize) -> anyhow::Result<()> {
        let svc = CommentService::new(self.storage.as_ref(), &self.user_id);
        let comments = svc.list(Some(&entry_id))?;
        let selected = selected.min(comments.len().saturating_sub(1));
        self.mode = AppMode::Comments {
            entry_id,
            comments,
            selected,
        };
        Ok(())
    }

    pub fn open_add_project_modal(&mut self) {
        self.mode = AppMode::AddProject(Form {
            fields: vec![
                Field::new("Name", ""),
                Field::new("Description (optional)", ""),
                Field::new("Color (optional, e.g. #ff0000)", ""),
            ],
            focused: 0,
        });
    }

    pub fn open_add_task_modal(&mut self) {
        let projects = self.project_names();
        self.mode = AppMode::AddTask(Form {
            fields: vec![
                Field::new("Project", "").with_completions(projects),
                Field::new("Name", ""),
                Field::new("Description (optional)", ""),
            ],
            focused: 0,
        });
    }

    pub fn add_project(
        &mut self,
        name: &str,
        description: &str,
        color: &str,
    ) -> anyhow::Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Project name is required"));
        }
        let desc = if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        };
        let col = if color.is_empty() {
            None
        } else {
            Some(color.to_string())
        };
        {
            let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            svc.add(name, desc, col)?;
        }
        self.refresh()?;
        self.status = Some((format!("Project '{name}' created."), false));
        Ok(())
    }

    pub fn add_task(&mut self, project: &str, name: &str, description: &str) -> anyhow::Result<()> {
        if project.is_empty() {
            return Err(anyhow::anyhow!("Project is required"));
        }
        if name.is_empty() {
            return Err(anyhow::anyhow!("Task name is required"));
        }
        let desc = if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        };
        {
            let svc = TaskService::new(self.storage.as_ref(), &self.user_id);
            svc.add(project, name, desc)?;
        }
        self.refresh()?;
        self.status = Some((format!("Task '{name}' created in '{project}'."), false));
        Ok(())
    }

    pub fn has_filter(&self) -> bool {
        self.filter_project_id.is_some() || self.filter_from.is_some() || self.filter_until.is_some()
    }

    pub fn entry_has_comments(&self, entry_id: &str) -> bool {
        self.entries_with_comments.contains(entry_id)
    }

    pub fn open_filter_modal(&mut self) {
        let projects = self.project_names();
        self.mode = AppMode::Filter(Form {
            fields: vec![
                Field::new("Project (empty = all)", &self.filter_project_name).with_completions(projects),
                Field::new("Date: today/yesterday/this week/YYYY-MM-DD/YYYY-MM-DD..YYYY-MM-DD", &self.filter_date_str),
            ],
            focused: 0,
        });
    }

    pub fn apply_filter(&mut self, project: &str, date_str: &str) -> anyhow::Result<()> {
        self.filter_project_name = project.to_string();
        self.filter_date_str = date_str.to_string();

        if project.is_empty() {
            self.filter_project_id = None;
        } else {
            let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            let proj = svc.resolve(project)?;
            self.filter_project_id = Some(proj.id);
        }

        let (from, until) = parse_date_filter(date_str)?;
        self.filter_from = from;
        self.filter_until = until;

        self.refresh()?;
        self.status = Some(("Filter applied.".into(), false));
        Ok(())
    }
}
