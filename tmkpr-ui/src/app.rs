use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use tmkpr_lib::{
    models::{
        comment::Comment,
        entry::{parse_tags, Entry, EntryFilter, UpdateEntry},
        project::{Project, UpdateProject},
        task::{Task, UpdateTask},
    },
    nlp::parser::{parse_datetime, TimeFormat},
    service::{CommentService, EntryService, ProjectService, TaskService, WeekReport},
    storage::Storage,
    ui_state::UiState,
};

use crate::form::{Field, Form};
use crate::theme::Theme;

use std::collections::HashMap;
use tmkpr_lib::config::ThemeConfig;

// Form field indices — prevents magic numbers in handlers
pub mod form_fields {
    pub mod start_modal {
        pub const PROJECT: usize = 0;
        pub const TASK: usize = 1;
        pub const NOTE: usize = 2;
        pub const TAGS: usize = 3;
    }

    pub mod edit_modal {
        pub const PROJECT: usize = 0;
        pub const TASK: usize = 1;
        pub const NOTE: usize = 2;
        pub const START: usize = 3;
        pub const END: usize = 4;
        pub const TAGS: usize = 5;
    }

    pub mod add_project {
        pub const NAME: usize = 0;
        pub const DESCRIPTION: usize = 1;
        pub const COLOR: usize = 2;
    }

    pub mod edit_project {
        pub const NAME: usize = 0;
        pub const DESCRIPTION: usize = 1;
        pub const COLOR: usize = 2;
    }

    pub mod add_task {
        pub const PROJECT: usize = 0;
        pub const NAME: usize = 1;
        pub const DESCRIPTION: usize = 2;
    }

    pub mod edit_task {
        pub const NAME: usize = 0;
        pub const DESCRIPTION: usize = 1;
    }

    pub mod filter {
        pub const PROJECT: usize = 0;
        pub const DATE: usize = 1;
    }

    pub mod filter_tasks {
        pub const PROJECT: usize = 0;
        pub const INCLUDE_ARCHIVED: usize = 1;
        pub const SHOW_COMPLETED: usize = 2;
    }

    pub mod filter_projects {
        pub const INCLUDE_ARCHIVED: usize = 0;
    }

    pub mod add_comment {
        pub const BODY: usize = 0;
    }

    pub mod edit_comment {
        pub const BODY: usize = 0;
    }
}

type DateRange = (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>);

fn parse_date_filter(s: &str) -> anyhow::Result<DateRange> {
    let s = s.trim();
    if s.is_empty() {
        return Ok((None, None));
    }

    fn naive_to_utc(naive: chrono::NaiveDateTime) -> chrono::DateTime<Utc> {
        Local
            .from_local_datetime(&naive)
            .unwrap()
            .with_timezone(&Utc)
    }

    match s {
        "today" => {
            let now = Local::now();
            let from = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            let until = (now.date_naive() + Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap();
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
            let until = (until_date + Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap();
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProjectSort {
    Name,
    NameDesc,
    Created,
    CreatedDesc,
}

impl ProjectSort {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectSort::Name => "Name (A-Z)",
            ProjectSort::NameDesc => "Name (Z-A)",
            ProjectSort::Created => "Oldest first",
            ProjectSort::CreatedDesc => "Newest first",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ProjectSort::Name => ProjectSort::NameDesc,
            ProjectSort::NameDesc => ProjectSort::Created,
            ProjectSort::Created => ProjectSort::CreatedDesc,
            ProjectSort::CreatedDesc => ProjectSort::Name,
        }
    }
}

#[derive(Clone, Default)]
pub struct ProjectFilter {
    pub hide_archived: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TaskSort {
    Name,
    NameDesc,
    Project,
    Created,
    CreatedDesc,
}

impl TaskSort {
    pub fn label(&self) -> &'static str {
        match self {
            TaskSort::Name => "Name (A-Z)",
            TaskSort::NameDesc => "Name (Z-A)",
            TaskSort::Project => "Project",
            TaskSort::Created => "Oldest first",
            TaskSort::CreatedDesc => "Newest first",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TaskSort::Name => TaskSort::NameDesc,
            TaskSort::NameDesc => TaskSort::Project,
            TaskSort::Project => TaskSort::Created,
            TaskSort::Created => TaskSort::CreatedDesc,
            TaskSort::CreatedDesc => TaskSort::Name,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EntrySort {
    StartDesc,
    Start,
    DurationDesc,
    Duration,
    Project,
}

impl EntrySort {
    pub fn label(&self) -> &'static str {
        match self {
            EntrySort::StartDesc => "Newest first",
            EntrySort::Start => "Oldest first",
            EntrySort::DurationDesc => "Longest first",
            EntrySort::Duration => "Shortest first",
            EntrySort::Project => "Project (A-Z)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            EntrySort::StartDesc => EntrySort::Start,
            EntrySort::Start => EntrySort::DurationDesc,
            EntrySort::DurationDesc => EntrySort::Duration,
            EntrySort::Duration => EntrySort::Project,
            EntrySort::Project => EntrySort::StartDesc,
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "Newest first" => Some(EntrySort::StartDesc),
            "Oldest first" => Some(EntrySort::Start),
            "Longest first" => Some(EntrySort::DurationDesc),
            "Shortest first" => Some(EntrySort::Duration),
            "Project (A-Z)" => Some(EntrySort::Project),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct TaskFilter {
    pub project_id: Option<String>,
    pub hide_archived: bool,
    pub hide_completed: bool,
}

impl Default for TaskFilter {
    fn default() -> Self {
        Self {
            project_id: None,
            hide_archived: true,
            hide_completed: true,
        }
    }
}

#[derive(Clone, Default)]
pub struct EntryFilterInput {
    pub project_name: String,
    pub date_str: String,
    pub project_id: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

impl EntryFilterInput {
    pub fn is_active(&self) -> bool {
        self.project_id.is_some() || self.from.is_some() || self.until.is_some()
    }
}

pub enum AppMode {
    Normal,
    Command {
        buf: String,
        completions: Vec<String>,
        completion_idx: Option<usize>,
        original_theme: Option<crate::theme::Theme>,
    },
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
    ManageProjects {
        projects: Vec<Project>,
        selected: usize,
    },
    EditProject {
        project_id: String,
        form: Form,
    },
    FilterProjects(Form),
    AddTask(Form),
    ManageTasks {
        tasks: Vec<Task>,
        selected: usize,
    },
    EditTask {
        task_id: String,
        form: Form,
    },
    Filter(Form),
    FilterTasks(Form),
    Comments {
        entry_id: String,
        comments: Vec<Comment>,
        selected: usize,
    },
    AddComment {
        entry_id: String,
        form: Form,
    },
    EditComment {
        entry_id: String,
        comment_id: String,
        form: Form,
    },
    ConfirmCreate {
        project: String,
        task: String,
        note: String,
        tags: String,
        create_project: bool,
        create_task: bool,
    },
    ConfirmDeleteProject {
        id: String,
        name: String,
    },
    Help,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ModeKind {
    Normal,
    Command,
    StartModal,
    EditModal,
    ConfirmDelete,
    AddProject,
    ManageProjects,
    EditProject,
    FilterProjects,
    AddTask,
    ManageTasks,
    EditTask,
    Filter,
    FilterTasks,
    Comments,
    AddComment,
    EditComment,
    ConfirmCreate,
    ConfirmDeleteProject,
    Help,
}

impl AppMode {
    pub fn kind(&self) -> ModeKind {
        match self {
            AppMode::Normal => ModeKind::Normal,
            AppMode::Command { .. } => ModeKind::Command,
            AppMode::StartModal(_) => ModeKind::StartModal,
            AppMode::EditModal { .. } => ModeKind::EditModal,
            AppMode::ConfirmDelete { .. } => ModeKind::ConfirmDelete,
            AppMode::AddProject(_) => ModeKind::AddProject,
            AppMode::ManageProjects { .. } => ModeKind::ManageProjects,
            AppMode::EditProject { .. } => ModeKind::EditProject,
            AppMode::FilterProjects(_) => ModeKind::FilterProjects,
            AppMode::AddTask(_) => ModeKind::AddTask,
            AppMode::ManageTasks { .. } => ModeKind::ManageTasks,
            AppMode::EditTask { .. } => ModeKind::EditTask,
            AppMode::Filter(_) => ModeKind::Filter,
            AppMode::FilterTasks(_) => ModeKind::FilterTasks,
            AppMode::Comments { .. } => ModeKind::Comments,
            AppMode::AddComment { .. } => ModeKind::AddComment,
            AppMode::EditComment { .. } => ModeKind::EditComment,
            AppMode::ConfirmCreate { .. } => ModeKind::ConfirmCreate,
            AppMode::ConfirmDeleteProject { .. } => ModeKind::ConfirmDeleteProject,
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
    pub entry_filter: EntryFilterInput,
    pub entries_with_comments: HashSet<String>,
    pub entry_sort: EntrySort,
    pub project_sort: ProjectSort,
    pub project_filter: ProjectFilter,
    pub task_sort: TaskSort,
    pub task_filter: TaskFilter,
    pub theme: Theme,
    pub themes: HashMap<String, ThemeConfig>,
    pub pending_open: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(
        storage: Box<dyn Storage>,
        user_id: String,
        theme: Theme,
        themes: HashMap<String, ThemeConfig>,
    ) -> Self {
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
            entry_filter: EntryFilterInput::default(),
            entries_with_comments: HashSet::new(),
            entry_sort: EntrySort::StartDesc,
            project_sort: ProjectSort::Name,
            project_filter: ProjectFilter::default(),
            task_sort: TaskSort::Name,
            task_filter: TaskFilter::default(),
            theme,
            themes,
            pending_open: None,
        }
    }

    pub fn enter_command_mode(&mut self) {
        self.mode = AppMode::Command {
            buf: String::new(),
            completions: vec![],
            completion_idx: None,
            original_theme: None,
        };
    }

    pub fn command_push(&mut self, c: char) {
        if let AppMode::Command { buf, .. } = &mut self.mode {
            buf.push(c);
        }
        self.update_command_completions();
    }

    pub fn command_pop(&mut self) {
        if let AppMode::Command { buf, .. } = &mut self.mode {
            buf.pop();
        }
        self.update_command_completions();
    }

    fn update_command_completions(&mut self) {
        let buf = match &self.mode {
            AppMode::Command { buf, .. } => buf.clone(),
            _ => return,
        };

        let new_completions = if let Some(rest) = buf.trim().strip_prefix("theme") {
            let filter = rest.trim_start().to_lowercase();
            let mut all: Vec<String> = crate::theme::Theme::builtin_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            for name in self.themes.keys() {
                if !all.contains(name) {
                    all.push(name.clone());
                }
            }
            all.sort();
            if filter.is_empty() {
                all
            } else {
                all.into_iter()
                    .filter(|n| n.to_lowercase().contains(&filter))
                    .collect()
            }
        } else {
            vec![]
        };

        if let AppMode::Command {
            completions,
            completion_idx,
            ..
        } = &mut self.mode
        {
            if *completions != new_completions {
                *completions = new_completions;
                *completion_idx = None;
            }
        }
    }

    pub fn command_tab(&mut self, forward: bool) {
        let has_completions = matches!(
            &self.mode,
            AppMode::Command { completions, .. } if !completions.is_empty()
        );
        if !has_completions {
            return;
        }

        // Save original theme on first tab press
        let has_original = matches!(
            &self.mode,
            AppMode::Command {
                original_theme: Some(_),
                ..
            }
        );
        if !has_original {
            let saved = self.theme.clone();
            if let AppMode::Command { original_theme, .. } = &mut self.mode {
                *original_theme = Some(saved);
            }
        }

        // Compute next index and theme name
        let (next_name, next_idx) = if let AppMode::Command {
            completions,
            completion_idx,
            ..
        } = &self.mode
        {
            let len = completions.len();
            let next = match *completion_idx {
                None => {
                    if forward {
                        0
                    } else {
                        len - 1
                    }
                }
                Some(idx) => {
                    if forward {
                        (idx + 1) % len
                    } else if idx == 0 {
                        len - 1
                    } else {
                        idx - 1
                    }
                }
            };
            (completions[next].clone(), next)
        } else {
            return;
        };

        if let AppMode::Command {
            completion_idx,
            buf,
            ..
        } = &mut self.mode
        {
            *completion_idx = Some(next_idx);
            *buf = format!("theme {next_name}");
        }

        let themes = self.themes.clone();
        self.theme = crate::theme::Theme::resolve(&next_name, &themes);
    }

    pub fn command_cancel(&mut self) {
        let original = if let AppMode::Command { original_theme, .. } = &mut self.mode {
            original_theme.take()
        } else {
            None
        };
        if let Some(t) = original {
            self.theme = t;
        }
        self.mode = AppMode::Normal;
    }

    pub fn execute_command(&mut self) -> anyhow::Result<()> {
        let buf = if let AppMode::Command { buf, .. } = &self.mode {
            buf.trim().to_string()
        } else {
            return Ok(());
        };

        // Confirm: drop original_theme (preview becomes permanent)
        self.mode = AppMode::Normal;

        let (cmd, arg) = match buf.split_once(' ') {
            Some((c, a)) => (c, a.trim()),
            None => (buf.as_str(), ""),
        };

        match cmd {
            "q" | "quit" => {
                self.running = false;
            }
            "theme" => {
                if arg.is_empty() {
                    self.status = Some(("Usage: theme <name>".into(), true));
                } else {
                    let themes = self.themes.clone();
                    self.theme = crate::theme::Theme::resolve(arg, &themes);
                    self.status = Some((format!("Theme set to '{arg}'."), false));
                }
            }
            "config-reload" => match tmkpr_lib::config::Config::load() {
                Ok(cfg) => {
                    let theme_name = cfg.display.theme.clone();
                    self.themes = cfg.themes.clone();
                    let themes = self.themes.clone();
                    self.theme = crate::theme::Theme::resolve(&theme_name, &themes);
                    self.status = Some(("Config reloaded.".into(), false));
                }
                Err(e) => {
                    self.status = Some((format!("Config reload failed: {e}"), true));
                }
            },
            "config-open" => match tmkpr_lib::config::config_path() {
                Ok(path) => {
                    self.pending_open = Some(path);
                }
                Err(e) => {
                    self.status = Some((format!("Cannot resolve config path: {e}"), true));
                }
            },
            "" => {}
            other => {
                self.status = Some((format!("Unknown command: '{other}'"), true));
            }
        }
        Ok(())
    }

    pub fn command_buf(&self) -> &str {
        if let AppMode::Command { buf, .. } = &self.mode {
            buf.as_str()
        } else {
            ""
        }
    }

    pub fn command_completion_state(&self) -> Option<(&[String], Option<usize>)> {
        if let AppMode::Command {
            completions,
            completion_idx,
            ..
        } = &self.mode
        {
            if !completions.is_empty() {
                return Some((completions.as_slice(), *completion_idx));
            }
        }
        None
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
                project_id: self.entry_filter.project_id.clone(),
                from: self.entry_filter.from,
                until: self.entry_filter.until,
                ..Default::default()
            })?;
            self.apply_entry_sort();
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
        self.tasks
            .iter()
            .filter(|t| !t.completed)
            .map(|t| t.name.clone())
            .collect()
    }

    pub fn task_names_for_project(&self, project_name: &str) -> Vec<String> {
        let project_id = self
            .projects
            .iter()
            .find(|p| p.name == project_name)
            .map(|p| p.id.as_str());

        if let Some(pid) = project_id {
            self.tasks
                .iter()
                .filter(|t| t.project_id == pid && !t.completed)
                .map(|t| t.name.clone())
                .collect()
        } else {
            vec![]
        }
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

    pub fn open_start_modal_from_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let entry = &self.entries[self.selected];
        let projects = self.project_names();
        let tasks = self.task_names();
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
        self.mode = AppMode::StartModal(Form {
            fields: vec![
                Field::new("Project", &project_val).with_completions(projects),
                Field::new("Task", &task_val).with_completions(tasks),
                Field::new("Note", &note_val),
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

    pub fn create_missing_and_start(
        &mut self,
        project: &str,
        task: &str,
        note: &str,
        tags_str: &str,
        create_project: bool,
        create_task: bool,
    ) -> anyhow::Result<()> {
        if create_project && !project.is_empty() {
            let proj_svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            proj_svc.add(project, None, None)?;
        }
        if create_task && !task.is_empty() && !project.is_empty() {
            let task_svc = TaskService::new(self.storage.as_ref(), &self.user_id);
            task_svc.add(project, task, None)?;
        }
        self.start_entry(project, task, note, tags_str)
    }

    #[allow(clippy::too_many_arguments)]
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

    pub fn fill_gaps(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        let id = self.entries[self.selected].id.clone();
        let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
        if !svc.fill_gaps(&id)? {
            self.status = Some(("No adjacent entries found.".into(), false));
        } else {
            self.status = Some(("Gaps filled.".into(), false));
        }
        self.refresh()?;
        Ok(())
    }

    pub fn fill_gaps_active(&mut self) -> anyhow::Result<()> {
        match self.active_entry.clone() {
            Some(e) => {
                let id = e.id.clone();
                let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
                if !svc.fill_gaps(&id)? {
                    self.status = Some(("No adjacent entries found.".into(), false));
                } else {
                    self.status = Some(("Gaps filled.".into(), false));
                }
                self.refresh()?;
                Ok(())
            }
            None => {
                self.status = Some(("No active entry.".into(), true));
                Ok(())
            }
        }
    }

    pub fn merge_with_next(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let id = self.entries[self.selected].id.clone();
        let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
        svc.merge_into_next(&id)?;

        self.refresh()?;
        self.status = Some(("Entries merged.".into(), false));
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
        self.entries_with_comments.insert(entry_id.clone());
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
        self.refresh_comments_mode(entry_id.clone(), 0)?;
        if let AppMode::Comments { comments, .. } = &self.mode {
            if comments.is_empty() {
                self.entries_with_comments.remove(&entry_id);
            }
        }
        self.status = Some(("Comment deleted.".into(), false));
        Ok(())
    }

    pub fn open_edit_comment(&mut self) -> anyhow::Result<()> {
        if let AppMode::Comments {
            entry_id,
            comments,
            selected,
        } = &self.mode
        {
            if comments.is_empty() {
                return Ok(());
            }
            let comment = &comments[*selected];
            self.mode = AppMode::EditComment {
                entry_id: entry_id.clone(),
                comment_id: comment.id.clone(),
                form: Form {
                    fields: vec![Field::new("Comment", &comment.body)],
                    focused: 0,
                },
            };
        }
        Ok(())
    }

    pub fn submit_edit_comment(&mut self, comment_id: String, body: String) -> anyhow::Result<()> {
        if body.is_empty() {
            return Err(anyhow::anyhow!("Comment cannot be empty"));
        }
        {
            let svc = CommentService::new(self.storage.as_ref(), &self.user_id);
            svc.edit(&comment_id, body)?;
        }
        self.status = Some(("Comment updated.".into(), false));
        Ok(())
    }

    pub fn cancel_edit_comment(&mut self) -> anyhow::Result<()> {
        if let AppMode::EditComment { entry_id, .. } = &self.mode {
            let entry_id = entry_id.clone();
            self.refresh_comments_mode(entry_id, 0)?;
        }
        Ok(())
    }

    pub fn refresh_comments_mode(
        &mut self,
        entry_id: String,
        selected: usize,
    ) -> anyhow::Result<()> {
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

    fn apply_project_sort_filter(&self, projects: Vec<Project>) -> Vec<Project> {
        let mut filtered = projects;

        if self.project_filter.hide_archived {
            filtered.retain(|p| !p.archived);
        }

        match self.project_sort {
            ProjectSort::Name => {
                filtered.sort_by(|a, b| a.name.cmp(&b.name));
            }
            ProjectSort::NameDesc => {
                filtered.sort_by(|a, b| b.name.cmp(&a.name));
            }
            ProjectSort::Created => {
                filtered.sort_by_key(|a| a.created_at);
            }
            ProjectSort::CreatedDesc => {
                filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at));
            }
        }

        filtered
    }

    pub fn open_manage_projects(&mut self) {
        let projects = self.apply_project_sort_filter(self.projects.clone());
        self.mode = AppMode::ManageProjects {
            projects,
            selected: 0,
        };
    }

    pub fn open_confirm_delete_project(&mut self) -> anyhow::Result<()> {
        let (id, name) = if let AppMode::ManageProjects { projects, selected } = &self.mode {
            if projects.is_empty() {
                return Ok(());
            }
            let p = &projects[*selected];
            (p.id.clone(), p.name.clone())
        } else {
            return Ok(());
        };

        let tasks = self.storage.list_tasks(&id, true)?;
        if !tasks.is_empty() {
            self.status = Some((
                format!(
                    "Cannot delete '{}': it has tasks. Delete or move them first.",
                    name
                ),
                true,
            ));
            return Ok(());
        }

        let entries = self.storage.list_entries(&EntryFilter {
            user_id: self.user_id.clone(),
            project_id: Some(id.clone()),
            include_active: true,
            ..Default::default()
        })?;
        if !entries.is_empty() {
            self.status = Some((format!("Cannot delete '{}': it has entries.", name), true));
            return Ok(());
        }

        self.mode = AppMode::ConfirmDeleteProject { id, name };
        Ok(())
    }

    pub fn delete_project(&mut self, id: &str, name: &str) -> anyhow::Result<()> {
        self.storage.delete_project(id)?;
        self.refresh()?;
        let projects = self.apply_project_sort_filter(self.projects.clone());
        self.mode = AppMode::ManageProjects {
            projects,
            selected: 0,
        };
        self.status = Some((format!("Project '{name}' deleted."), false));
        Ok(())
    }

    pub fn open_project_filter_modal(&mut self) {
        self.mode = AppMode::FilterProjects(Form {
            fields: vec![Field::new(
                "Show archived projects? (y/n)",
                if self.project_filter.hide_archived {
                    "n"
                } else {
                    "y"
                },
            )],
            focused: 0,
        });
    }

    pub fn select_next_project(&mut self) {
        if let AppMode::ManageProjects { projects, selected } = &mut self.mode {
            *selected = (*selected + 1).min(projects.len().saturating_sub(1));
        }
    }

    pub fn select_prev_project(&mut self) {
        if let AppMode::ManageProjects { selected, .. } = &mut self.mode {
            *selected = selected.saturating_sub(1);
        }
    }

    pub fn open_edit_selected_project(&mut self) -> anyhow::Result<()> {
        if let AppMode::ManageProjects { projects, selected } = &self.mode {
            if projects.is_empty() {
                return Ok(());
            }
            let proj = &projects[*selected];
            let form = Form {
                fields: vec![
                    Field::new("Name", proj.name.clone()),
                    Field::new(
                        "Description (optional)",
                        proj.description.clone().unwrap_or_default(),
                    ),
                    Field::new(
                        "Color (optional, e.g. #ff0000)",
                        proj.color.clone().unwrap_or_default(),
                    ),
                ],
                focused: 0,
            };
            self.mode = AppMode::EditProject {
                project_id: proj.id.clone(),
                form,
            };
        }
        Ok(())
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

    pub fn submit_edit_project(
        &mut self,
        project_id: String,
        name: &str,
        description: &str,
        color: &str,
    ) -> anyhow::Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Project name is required"));
        }

        let update = UpdateProject {
            name: Some(name.to_string()),
            description: Some(if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            }),
            color: Some(if color.is_empty() {
                None
            } else {
                Some(color.to_string())
            }),
            archived: None,
        };

        self.storage.update_project(&project_id, update)?;
        self.refresh()?;
        let projects = self.apply_project_sort_filter(self.projects.clone());
        let selected = 0; // Reset on edit for simplicity; could preserve position by finding edited project in list
        self.mode = AppMode::ManageProjects { projects, selected };
        self.status = Some((format!("Project '{name}' updated."), false));
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

    fn apply_task_sort_filter(&self, tasks: Vec<Task>) -> Vec<Task> {
        let mut filtered = tasks;

        if let Some(pid) = &self.task_filter.project_id {
            filtered.retain(|t| &t.project_id == pid);
        }

        if self.task_filter.hide_archived {
            filtered.retain(|t| !t.archived);
        }

        if self.task_filter.hide_completed {
            filtered.retain(|t| !t.completed);
        }

        match self.task_sort {
            TaskSort::Name => {
                filtered.sort_by(|a, b| a.name.cmp(&b.name));
            }
            TaskSort::NameDesc => {
                filtered.sort_by(|a, b| b.name.cmp(&a.name));
            }
            TaskSort::Project => {
                filtered.sort_by(|a, b| {
                    self.project_name(&a.project_id)
                        .cmp(self.project_name(&b.project_id))
                });
            }
            TaskSort::Created => {
                filtered.sort_by_key(|a| a.created_at);
            }
            TaskSort::CreatedDesc => {
                filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at));
            }
        }

        filtered
    }

    pub fn open_manage_tasks(&mut self) {
        let tasks = self.apply_task_sort_filter(self.tasks.clone());
        self.mode = AppMode::ManageTasks { tasks, selected: 0 };
    }

    pub fn open_task_filter_modal(&mut self) {
        let projects = self.project_names();
        let project_filter = self
            .task_filter
            .project_id
            .as_ref()
            .and_then(|pid| {
                self.projects
                    .iter()
                    .find(|p| &p.id == pid)
                    .map(|p| p.name.clone())
            })
            .unwrap_or_default();

        self.mode = AppMode::FilterTasks(Form {
            fields: vec![
                Field::new("Filter by project (empty = all)", &project_filter)
                    .with_completions(projects),
                Field::new(
                    "Show archived tasks? (y/n)",
                    if self.task_filter.hide_archived {
                        "n"
                    } else {
                        "y"
                    },
                ),
                Field::new(
                    "Show completed tasks? (y/n)",
                    if self.task_filter.hide_completed {
                        "n"
                    } else {
                        "y"
                    },
                ),
            ],
            focused: 0,
        });
    }

    pub fn select_next_task(&mut self) {
        if let AppMode::ManageTasks { tasks, selected } = &mut self.mode {
            *selected = (*selected + 1).min(tasks.len().saturating_sub(1));
        }
    }

    pub fn select_prev_task(&mut self) {
        if let AppMode::ManageTasks { selected, .. } = &mut self.mode {
            *selected = selected.saturating_sub(1);
        }
    }

    pub fn open_edit_selected_task(&mut self) -> anyhow::Result<()> {
        if let AppMode::ManageTasks { tasks, selected } = &self.mode {
            if tasks.is_empty() {
                return Ok(());
            }
            let task = &tasks[*selected];
            let form = Form {
                fields: vec![
                    Field::new("Name", task.name.clone()),
                    Field::new(
                        "Description (optional)",
                        task.description.clone().unwrap_or_default(),
                    ),
                ],
                focused: 0,
            };
            self.mode = AppMode::EditTask {
                task_id: task.id.clone(),
                form,
            };
        }
        Ok(())
    }

    pub fn submit_edit_task(
        &mut self,
        task_id: String,
        name: &str,
        description: &str,
    ) -> anyhow::Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Task name is required"));
        }

        let update = UpdateTask {
            name: Some(name.to_string()),
            description: Some(if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            }),
            archived: None,
            completed: None,
            project_id: None,
        };

        self.storage.update_task(&task_id, update)?;
        self.refresh()?;
        let tasks = self.apply_task_sort_filter(self.tasks.clone());
        let selected = 0; // Reset on edit for simplicity; could preserve position by finding edited task in list
        self.mode = AppMode::ManageTasks { tasks, selected };
        self.status = Some((format!("Task '{name}' updated."), false));
        Ok(())
    }

    pub fn toggle_complete_selected_task(&mut self) -> anyhow::Result<()> {
        let (task_id, currently_completed) =
            if let AppMode::ManageTasks { tasks, selected } = &self.mode {
                if *selected < tasks.len() {
                    let task = &tasks[*selected];
                    (task.id.clone(), task.completed)
                } else {
                    return Ok(());
                }
            } else {
                return Ok(());
            };

        self.storage.update_task(
            &task_id,
            UpdateTask {
                completed: Some(!currently_completed),
                ..Default::default()
            },
        )?;
        self.refresh()?;
        let tasks = self.apply_task_sort_filter(self.tasks.clone());
        let selected = 0;
        self.mode = AppMode::ManageTasks { tasks, selected };
        let msg = if !currently_completed {
            "Task marked completed."
        } else {
            "Task reactivated."
        };
        self.status = Some((msg.to_string(), false));
        Ok(())
    }

    pub fn delete_selected_task(&mut self) -> anyhow::Result<()> {
        let (project_name, task_name) = if let AppMode::ManageTasks { tasks, selected } = &self.mode
        {
            if *selected < tasks.len() {
                let task = &tasks[*selected];
                (
                    self.project_name(&task.project_id).to_string(),
                    task.name.clone(),
                )
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        };

        let svc = TaskService::new(self.storage.as_ref(), &self.user_id);
        svc.delete(&project_name, &task_name, false)?;
        self.refresh()?;
        let tasks = self.apply_task_sort_filter(self.tasks.clone());
        let selected = 0; // Reset on delete; could preserve position if list still non-empty
        self.mode = AppMode::ManageTasks { tasks, selected };
        self.status = Some((format!("Task '{}' deleted.", task_name), false));
        Ok(())
    }

    pub fn has_filter(&self) -> bool {
        self.entry_filter.is_active()
    }

    pub fn entry_has_comments(&self, entry_id: &str) -> bool {
        self.entries_with_comments.contains(entry_id)
    }

    fn apply_entry_sort(&mut self) {
        match self.entry_sort {
            EntrySort::StartDesc => {
                self.entries
                    .sort_by_key(|b| std::cmp::Reverse(b.started_at));
            }
            EntrySort::Start => {
                self.entries.sort_by_key(|a| a.started_at);
            }
            EntrySort::DurationDesc => {
                self.entries.sort_by(|a, b| {
                    let a_secs = a.elapsed().num_seconds();
                    let b_secs = b.elapsed().num_seconds();
                    b_secs.cmp(&a_secs)
                });
            }
            EntrySort::Duration => {
                self.entries.sort_by(|a, b| {
                    let a_secs = a.elapsed().num_seconds();
                    let b_secs = b.elapsed().num_seconds();
                    a_secs.cmp(&b_secs)
                });
            }
            EntrySort::Project => {
                let project_map: std::collections::HashMap<&str, &str> = self
                    .projects
                    .iter()
                    .map(|p| (p.id.as_str(), p.name.as_str()))
                    .collect();
                self.entries.sort_by(|a, b| {
                    let a_proj = a
                        .project_id
                        .as_deref()
                        .and_then(|pid| project_map.get(pid))
                        .copied()
                        .unwrap_or("?");
                    let b_proj = b
                        .project_id
                        .as_deref()
                        .and_then(|pid| project_map.get(pid))
                        .copied()
                        .unwrap_or("?");
                    a_proj.cmp(b_proj)
                });
            }
        }
    }

    pub fn open_filter_modal(&mut self) {
        let projects = self.project_names();
        self.mode = AppMode::Filter(Form {
            fields: vec![
                Field::new("Project (empty = all)", &self.entry_filter.project_name)
                    .with_completions(projects),
                Field::new(
                    "Date: today/yesterday/this week/YYYY-MM-DD/YYYY-MM-DD..YYYY-MM-DD",
                    &self.entry_filter.date_str,
                ),
            ],
            focused: 0,
        });
    }

    pub fn apply_filter(&mut self, project: &str, date_str: &str) -> anyhow::Result<()> {
        self.apply_filter_internal(project, date_str, true)
    }

    fn apply_filter_internal(
        &mut self,
        project: &str,
        date_str: &str,
        show_message: bool,
    ) -> anyhow::Result<()> {
        self.entry_filter.project_name = project.to_string();
        self.entry_filter.date_str = date_str.to_string();

        if project.is_empty() {
            self.entry_filter.project_id = None;
        } else {
            let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            let proj = svc.resolve(project)?;
            self.entry_filter.project_id = Some(proj.id);
        }

        let (from, until) = parse_date_filter(date_str)?;
        self.entry_filter.from = from;
        self.entry_filter.until = until;

        self.refresh()?;
        if show_message && !project.is_empty() {
            self.status = Some(("Filter applied.".into(), false));
        } else if show_message {
            self.status = None;
        }
        Ok(())
    }

    pub fn load_ui_state(&mut self) -> anyhow::Result<()> {
        if let Ok(state) = UiState::load() {
            if !state.entry_filter_project.is_empty() || !state.entry_filter_date.is_empty() {
                self.apply_filter(&state.entry_filter_project, &state.entry_filter_date)?;
            }
            if !state.entry_sort.is_empty() {
                if let Some(sort) = EntrySort::from_label(&state.entry_sort) {
                    self.entry_sort = sort;
                    self.apply_entry_sort();
                }
            }
        }
        Ok(())
    }

    pub fn save_ui_state(&self) -> anyhow::Result<()> {
        let state = UiState {
            entry_sort: self.entry_sort.label().to_string(),
            entry_filter_project: self.entry_filter.project_name.clone(),
            entry_filter_date: self.entry_filter.date_str.clone(),
        };
        state.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use tmkpr_lib::models::comment::NewComment;
    use tmkpr_lib::models::entry::NewEntry;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::storage::sqlite::SqliteStorage;

    fn make_app() -> App {
        let storage = SqliteStorage::open_in_memory().unwrap();
        App::new(
            Box::new(storage),
            LOCAL_USER_ID.to_string(),
            crate::theme::Theme::from_name("default"),
            HashMap::new(),
        )
    }

    fn finished(
        app: &App,
        note: Option<&str>,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Entry {
        app.storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: note.map(str::to_string),
                started_at,
                finished_at: Some(finished_at),
                tags: vec![],
            })
            .unwrap()
    }

    fn active(app: &App, note: Option<&str>, started_at: DateTime<Utc>) -> Entry {
        app.storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: note.map(str::to_string),
                started_at,
                finished_at: None,
                tags: vec![],
            })
            .unwrap()
    }

    fn select(app: &mut App, id: &str) {
        app.selected = app.entries.iter().position(|e| e.id == id).unwrap();
    }

    #[test]
    fn parse_date_filter_empty_returns_none() {
        let (from, until) = parse_date_filter("").unwrap();
        assert!(from.is_none());
        assert!(until.is_none());
    }

    #[test]
    fn parse_date_filter_whitespace_returns_none() {
        let (from, until) = parse_date_filter("   ").unwrap();
        assert!(from.is_none());
        assert!(until.is_none());
    }

    #[test]
    fn parse_date_filter_today() {
        let (from, until) = parse_date_filter("today").unwrap();
        assert!(from.is_some());
        assert!(until.is_some());
        let from = from.unwrap();
        let until = until.unwrap();
        // Duration should be approximately 24 hours
        let duration = until.signed_duration_since(from);
        assert_eq!(duration.num_hours(), 24);
    }

    #[test]
    fn parse_date_filter_yesterday() {
        let (from, until) = parse_date_filter("yesterday").unwrap();
        assert!(from.is_some());
        assert!(until.is_some());
        let from = from.unwrap();
        let until = until.unwrap();
        // Duration should be approximately 24 hours
        let duration = until.signed_duration_since(from);
        assert_eq!(duration.num_hours(), 24);
    }

    #[test]
    fn parse_date_filter_this_week() {
        let (from, until) = parse_date_filter("this week").unwrap();
        assert!(from.is_some());
        assert!(until.is_some());
        let from = from.unwrap();
        let until = until.unwrap();
        // Duration should be approximately 7 days
        let duration = until.signed_duration_since(from);
        assert_eq!(duration.num_days(), 7);
    }

    #[test]
    fn parse_date_filter_specific_date() {
        let (from, until) = parse_date_filter("2024-05-15").unwrap();
        assert!(from.is_some());
        assert!(until.is_some());
        let from = from.unwrap();
        let until = until.unwrap();
        // Duration should be 24 hours
        let duration = until.signed_duration_since(from);
        assert_eq!(duration.num_hours(), 24);
    }

    #[test]
    fn parse_date_filter_date_range() {
        let (from, until) = parse_date_filter("2024-05-15..2024-05-18").unwrap();
        assert!(from.is_some());
        assert!(until.is_some());
        let from = from.unwrap();
        let until = until.unwrap();
        // Duration should be approximately 4 days (from May 15 to May 18 inclusive)
        let duration = until.signed_duration_since(from);
        assert_eq!(duration.num_days(), 4);
    }

    #[test]
    fn parse_date_filter_invalid_date_fails() {
        let result = parse_date_filter("invalid-date");
        assert!(result.is_err());
    }

    #[test]
    fn project_sort_cycles() {
        let mut sort = ProjectSort::Name;
        assert_eq!(sort, ProjectSort::Name);
        sort = sort.next();
        assert_eq!(sort, ProjectSort::NameDesc);
        sort = sort.next();
        assert_eq!(sort, ProjectSort::Created);
        sort = sort.next();
        assert_eq!(sort, ProjectSort::CreatedDesc);
        sort = sort.next();
        assert_eq!(sort, ProjectSort::Name);
    }

    #[test]
    fn task_sort_cycles() {
        let mut sort = TaskSort::Name;
        assert_eq!(sort, TaskSort::Name);
        sort = sort.next();
        assert_eq!(sort, TaskSort::NameDesc);
        sort = sort.next();
        assert_eq!(sort, TaskSort::Project);
        sort = sort.next();
        assert_eq!(sort, TaskSort::Created);
        sort = sort.next();
        assert_eq!(sort, TaskSort::CreatedDesc);
        sort = sort.next();
        assert_eq!(sort, TaskSort::Name);
    }

    #[test]
    fn entry_filter_input_is_active() {
        let filter = EntryFilterInput::default();
        assert!(!filter.is_active());

        let filter = EntryFilterInput {
            project_id: Some("proj-1".to_string()),
            ..Default::default()
        };
        assert!(filter.is_active());

        let filter = EntryFilterInput {
            from: Some(Utc::now()),
            ..Default::default()
        };
        assert!(filter.is_active());

        let filter = EntryFilterInput {
            until: Some(Utc::now()),
            ..Default::default()
        };
        assert!(filter.is_active());
    }

    // ── merge_with_next ───────────────────────────────────────────────────────

    #[test]
    fn merge_adopts_first_start_time() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, None, t0, t1);
        finished(&app, None, t1, t2);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        // Only one entry should remain.
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].started_at, t0);
        assert_eq!(app.entries[0].finished_at, Some(t2));
    }

    #[test]
    fn merge_notes_prepended_when_distinct() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, Some("alpha"), t0, t1);
        finished(&app, Some("beta"), t1, t2);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        assert_eq!(app.entries[0].note.as_deref(), Some("alpha\nbeta"));
    }

    #[test]
    fn merge_notes_unchanged_when_identical() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, Some("same"), t0, t1);
        finished(&app, Some("same"), t1, t2);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        assert_eq!(app.entries[0].note.as_deref(), Some("same"));
    }

    #[test]
    fn merge_note_from_first_when_second_empty() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, Some("only first"), t0, t1);
        finished(&app, None, t1, t2);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        assert_eq!(app.entries[0].note.as_deref(), Some("only first"));
    }

    #[test]
    fn merge_note_kept_when_first_empty() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, None, t0, t1);
        finished(&app, Some("only second"), t1, t2);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        assert_eq!(app.entries[0].note.as_deref(), Some("only second"));
    }

    #[test]
    fn merge_moves_comments_to_second() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        let first = finished(&app, None, t0, t1);
        let second = finished(&app, None, t1, t2);

        app.storage
            .create_comment(NewComment {
                entry_id: first.id.clone(),
                body: "moved comment".to_string(),
            })
            .unwrap();

        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        let comments = app.storage.list_comments(&second.id).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].body, "moved comment");

        // First entry is gone; its original comments were cascade-deleted.
        let first_comments = app.storage.list_comments(&first.id).unwrap();
        assert!(first_comments.is_empty());
    }

    #[test]
    fn merge_works_when_second_is_active() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(2);
        let t1 = now - Duration::hours(1);

        let first = finished(&app, Some("note"), t0, t1);
        active(&app, None, t1);
        app.refresh().unwrap();
        select(&mut app, &first.id);

        app.merge_with_next().unwrap();

        // First entry gone, active entry adopts first's start time and note.
        assert!(app.entries.is_empty());
        let merged = app.active_entry.as_ref().unwrap();
        assert_eq!(merged.started_at, t0);
        assert_eq!(merged.note.as_deref(), Some("note"));
    }

    #[test]
    fn merge_errors_when_no_successor() {
        let mut app = make_app();
        let now = Utc::now();
        let first = finished(
            &app,
            None,
            now - Duration::hours(2),
            now - Duration::hours(1),
        );
        app.refresh().unwrap();
        select(&mut app, &first.id);

        let err = app.merge_with_next().unwrap_err();
        assert!(err.to_string().contains("no subsequent entry"));
    }

    #[test]
    fn merge_skips_entries_with_different_project_or_task() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1);

        // Create a real project so the foreign key constraint is satisfied.
        let proj = ProjectService::new(app.storage.as_ref(), LOCAL_USER_ID)
            .add("other-proj", None, None)
            .unwrap();

        // First entry: no project. Second entry: has a project — different from first.
        let first = finished(&app, None, t0, t1);
        app.storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: Some(proj.id.clone()),
                task_id: None,
                note: None,
                started_at: t1,
                finished_at: Some(t2),
                tags: vec![],
            })
            .unwrap();

        app.refresh().unwrap();
        select(&mut app, &first.id);

        let err = app.merge_with_next().unwrap_err();
        assert!(err.to_string().contains("no subsequent entry"));
    }

    // ── fill_gaps ─────────────────────────────────────────────────────────────

    #[test]
    fn fill_gaps_extends_both_ends() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(5);
        let t1 = now - Duration::hours(4);
        let t2 = now - Duration::hours(3); // gap
        let t3 = now - Duration::hours(2);
        let t4 = now - Duration::hours(1); // gap
        let t5 = now;

        finished(&app, None, t0, t1); // prior
        let mid = finished(&app, None, t2, t3); // selected (has gap before and after)
        finished(&app, None, t4, t5); // subsequent

        app.refresh().unwrap();
        select(&mut app, &mid.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == mid.id).unwrap();
        assert_eq!(updated.started_at, t1);
        assert_eq!(updated.finished_at, Some(t4));
    }

    #[test]
    fn fill_gaps_extends_start_only_when_no_subsequent() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1); // gap

        finished(&app, None, t0, t1); // prior
        let target = finished(&app, None, t2, now);

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == target.id).unwrap();
        assert_eq!(updated.started_at, t1);
        assert_eq!(updated.finished_at, Some(now));
    }

    #[test]
    fn fill_gaps_extends_end_only_when_no_prior() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2); // gap
        let t2 = now - Duration::hours(1);

        let target = finished(&app, None, t0, t1);
        finished(&app, None, t2, now); // subsequent

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == target.id).unwrap();
        assert_eq!(updated.started_at, t0);
        assert_eq!(updated.finished_at, Some(t2));
    }

    #[test]
    fn fill_gaps_no_adjacent_entries_leaves_status_message() {
        let mut app = make_app();
        let now = Utc::now();
        let target = finished(
            &app,
            None,
            now - Duration::hours(2),
            now - Duration::hours(1),
        );

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        assert!(app
            .status
            .as_ref()
            .unwrap()
            .0
            .contains("No adjacent entries found"));
    }

    #[test]
    fn fill_gaps_active_extends_start_to_prior_finished_at() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1); // gap

        finished(&app, None, t0, t1); // prior
        active(&app, None, t2); // active entry with gap before it

        app.refresh().unwrap();
        app.fill_gaps_active().unwrap();

        let merged = app.active_entry.as_ref().unwrap();
        assert_eq!(merged.started_at, t1);
    }

    #[test]
    fn fill_gaps_does_not_extend_start_when_prior_is_previous_day() {
        let mut app = make_app();
        let yesterday_17h = {
            let yesterday = Local::now().date_naive() - chrono::Duration::days(1);
            Local
                .from_local_datetime(&yesterday.and_hms_opt(17, 0, 0).unwrap())
                .unwrap()
                .with_timezone(&Utc)
        };
        let today_9am = {
            let today = Local::now().date_naive();
            Local
                .from_local_datetime(&today.and_hms_opt(9, 0, 0).unwrap())
                .unwrap()
                .with_timezone(&Utc)
        };

        // Prior entry is clearly on the previous day.
        finished(
            &app,
            None,
            yesterday_17h - Duration::hours(1),
            yesterday_17h,
        );

        let target_start = today_9am;
        let target_end = today_9am + Duration::hours(1);
        let target = finished(&app, None, target_start, target_end);

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == target.id).unwrap();
        // Start must NOT change (first entry of the day); end has no same-day subsequent.
        assert_eq!(updated.started_at, target_start);
        assert_eq!(updated.finished_at, Some(target_end));
    }

    #[test]
    fn fill_gaps_does_not_extend_end_when_subsequent_is_next_day() {
        let mut app = make_app();
        // Place the target entry clearly within today and the subsequent entry tomorrow.
        let today_noon = {
            let today = Local::now().date_naive();
            Local
                .from_local_datetime(&today.and_hms_opt(12, 0, 0).unwrap())
                .unwrap()
                .with_timezone(&Utc)
        };
        let tomorrow_9am = today_noon + Duration::hours(21); // next day

        let prior_end = today_noon - Duration::hours(2);
        let prior_start = today_noon - Duration::hours(3);
        finished(&app, None, prior_start, prior_end); // prior, same day

        let target = finished(&app, None, today_noon - Duration::hours(1), today_noon);
        finished(&app, None, tomorrow_9am, tomorrow_9am + Duration::hours(1)); // next-day entry

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == target.id).unwrap();
        // Start extends to prior's end; finish must NOT change (last entry of the day).
        assert_eq!(updated.started_at, prior_end);
        assert_eq!(updated.finished_at, Some(today_noon));
    }

    #[test]
    fn fill_gaps_active_uses_active_entry_as_subsequent_for_selected() {
        let mut app = make_app();
        let now = Utc::now();
        let t0 = now - Duration::hours(3);
        let t1 = now - Duration::hours(2);
        let t2 = now - Duration::hours(1); // gap before active

        let target = finished(&app, None, t0, t1);
        active(&app, None, t2); // active entry is the subsequent

        app.refresh().unwrap();
        select(&mut app, &target.id);
        app.fill_gaps().unwrap();

        let updated = app.entries.iter().find(|e| e.id == target.id).unwrap();
        assert_eq!(updated.finished_at, Some(t2));
    }

    // ── task completion ───────────────────────────────────────────────────────

    fn setup_project_and_task(app: &App) -> (String, String) {
        use tmkpr_lib::models::project::NewProject;
        use tmkpr_lib::models::task::NewTask;
        let p = app
            .storage
            .create_project(NewProject {
                user_id: LOCAL_USER_ID.to_string(),
                name: "proj".to_string(),
                description: None,
                color: None,
            })
            .unwrap();
        let t = app
            .storage
            .create_task(NewTask {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: p.id.clone(),
                name: "task".to_string(),
                description: None,
            })
            .unwrap();
        (p.id, t.id)
    }

    #[test]
    fn toggle_complete_marks_task_done() {
        let mut app = make_app();
        let (_pid, tid) = setup_project_and_task(&app);
        app.refresh().unwrap();
        app.open_manage_tasks();
        app.toggle_complete_selected_task().unwrap();
        assert!(app.storage.get_task(&tid).unwrap().completed);
        assert_eq!(app.status.as_ref().unwrap().0, "Task marked completed.");
    }

    #[test]
    fn toggle_complete_reactivates_already_completed_task() {
        let mut app = make_app();
        let (_pid, tid) = setup_project_and_task(&app);
        // pre-mark as completed
        use tmkpr_lib::models::task::UpdateTask;
        app.storage
            .update_task(
                &tid,
                UpdateTask {
                    completed: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        app.refresh().unwrap();
        app.open_manage_tasks();
        // Show completed tasks to test toggling them back
        app.task_filter.hide_completed = false;
        let tasks = app.apply_task_sort_filter(app.tasks.clone());
        app.mode = AppMode::ManageTasks { tasks, selected: 0 };
        app.toggle_complete_selected_task().unwrap();
        assert!(!app.storage.get_task(&tid).unwrap().completed);
        assert_eq!(app.status.as_ref().unwrap().0, "Task reactivated.");
    }

    #[test]
    fn toggle_complete_noop_when_no_tasks() {
        let mut app = make_app();
        app.refresh().unwrap();
        app.open_manage_tasks();
        // should not panic or error
        app.toggle_complete_selected_task().unwrap();
    }

    #[test]
    fn task_names_excludes_completed() {
        let mut app = make_app();
        let (_pid, tid) = setup_project_and_task(&app);
        use tmkpr_lib::models::task::UpdateTask;
        app.storage
            .update_task(
                &tid,
                UpdateTask {
                    completed: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        app.refresh().unwrap();
        assert!(app.task_names().is_empty());
    }

    #[test]
    fn task_names_includes_active_tasks() {
        let mut app = make_app();
        setup_project_and_task(&app);
        app.refresh().unwrap();
        assert_eq!(app.task_names(), vec!["task"]);
    }
}
