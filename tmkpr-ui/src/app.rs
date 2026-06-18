use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc, Weekday};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use tmkpr_lib::{
    config::Config,
    models::{
        comment::Comment,
        entry::{parse_tags, Entry, EntryFilter, UpdateEntry},
        project::{Project, UpdateProject},
        task::{Task, UpdateTask},
    },
    nlp::parser::{parse_datetime, TimeFormat},
    obsidian_logger,
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

    pub mod edit_event_modal {
        pub const PROJECT: usize = 0;
        pub const TASK: usize = 1;
        pub const NOTE: usize = 2;
        pub const TIME: usize = 3;
        pub const TAGS: usize = 4;
    }

    pub mod add_event_modal {
        pub const PROJECT: usize = 0;
        pub const TASK: usize = 1;
        pub const NOTE: usize = 2;
        pub const TIME: usize = 3;
        pub const TAGS: usize = 4;
    }

    pub mod add_manual_entry {
        pub const PROJECT: usize = 0;
        pub const TASK: usize = 1;
        pub const NOTE: usize = 2;
        pub const START: usize = 3;
        pub const END: usize = 4;
        pub const TAGS: usize = 5;
        pub const SNAP_TO_EXISTING: usize = 6;
    }
}

type DateRange = (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>);

fn weekday_to_def(w: chrono::Weekday) -> tmkpr_lib::config::WeekdayDef {
    match w {
        chrono::Weekday::Mon => tmkpr_lib::config::WeekdayDef::Mon,
        chrono::Weekday::Tue => tmkpr_lib::config::WeekdayDef::Tue,
        chrono::Weekday::Wed => tmkpr_lib::config::WeekdayDef::Wed,
        chrono::Weekday::Thu => tmkpr_lib::config::WeekdayDef::Thu,
        chrono::Weekday::Fri => tmkpr_lib::config::WeekdayDef::Fri,
        chrono::Weekday::Sat => tmkpr_lib::config::WeekdayDef::Sat,
        chrono::Weekday::Sun => tmkpr_lib::config::WeekdayDef::Sun,
    }
}

fn parse_date_filter(s: &str, week_start: chrono::Weekday) -> anyhow::Result<DateRange> {
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
            let start_num = week_start.num_days_from_monday();
            let current_num = weekday.num_days_from_monday();
            let days_since_start = (current_num + 7 - start_num) % 7;
            let week_start = now.date_naive() - Duration::days(days_since_start as i64);
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
        _ if s.contains('/') && s.matches('/').count() == 1 => Err(anyhow::anyhow!(
            "Date range uses '..' not '/'. Try: YYYY-MM-DD..YYYY-MM-DD"
        )),
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
    EditEventModal {
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
    AddManualEntry(Form),
    AddEventModal(Form),
    Help {
        scroll: u16,
    },
    Settings {
        cursor: usize,
        theme_names: Vec<String>,
        theme_idx: usize,
        date_fmt_idx: usize,
        week_start: chrono::Weekday,
        obs_enabled: bool,
        obs_vault: String,
        obs_activity: String,
        obs_comment: String,
        text_editing: bool,
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum ModeKind {
    Normal,
    Command,
    StartModal,
    EditModal,
    EditEventModal,
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
    AddManualEntry,
    AddEventModal,
    Help,
    Settings,
}

impl AppMode {
    pub fn kind(&self) -> ModeKind {
        match self {
            AppMode::Normal => ModeKind::Normal,
            AppMode::Command { .. } => ModeKind::Command,
            AppMode::StartModal(_) => ModeKind::StartModal,
            AppMode::EditModal { .. } => ModeKind::EditModal,
            AppMode::EditEventModal { .. } => ModeKind::EditEventModal,
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
            AppMode::AddManualEntry(_) => ModeKind::AddManualEntry,
            AppMode::AddEventModal(_) => ModeKind::AddEventModal,
            AppMode::Help { .. } => ModeKind::Help,
            AppMode::Settings { .. } => ModeKind::Settings,
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
    pub theme_name: String,
    pub themes: HashMap<String, ThemeConfig>,
    pub pending_open: Option<std::path::PathBuf>,
    pub date_format: String,
    pub week_start: chrono::Weekday,
    pub displayed_week_year: i32,
    pub displayed_week_num: u32,
    pub config: Config,
}

/// Display name → chrono format string pairs for the `:set date-format` command.
pub const DATE_FORMAT_PRESETS: &[(&str, &str)] = &[
    ("YYYY-MM-DD HH:MM", "%Y-%m-%d %H:%M"),
    ("DD-MM-YYYY HH:MM", "%d-%m-%Y %H:%M"),
    ("MM-DD-YYYY HH:MM", "%m-%d-%Y %H:%M"),
];

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Box<dyn Storage>,
        user_id: String,
        theme_name: String,
        theme: Theme,
        themes: HashMap<String, ThemeConfig>,
        date_format: String,
        week_start: chrono::Weekday,
        config: Config,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let now = Local::now();
        let iso_week = now.iso_week();
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
            theme_name,
            themes,
            pending_open: None,
            date_format,
            week_start,
            displayed_week_year: iso_week.year(),
            displayed_week_num: iso_week.week(),
            config,
        }
    }

    pub fn enter_command_mode(&mut self) {
        self.mode = AppMode::Command {
            buf: String::new(),
            completions: vec![],
            completion_idx: None,
            original_theme: None,
        };
        self.update_command_completions();
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

        let trimmed = buf.trim();
        let new_completions = if trimmed == "theme" || trimmed.starts_with("theme ") {
            // "theme" or "theme <arg>" — complete theme names.
            let filter = trimmed
                .strip_prefix("theme")
                .unwrap_or("")
                .trim_start()
                .to_lowercase();
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
        } else if trimmed == "set date-format" || trimmed.starts_with("set date-format ") {
            // "set date-format <variant>" — complete format presets.
            DATE_FORMAT_PRESETS
                .iter()
                .map(|(display, _)| display.to_string())
                .collect()
        } else if trimmed == "set" || trimmed.starts_with("set ") {
            // "set" or "set <setting>" — complete setting names.
            let filter = trimmed
                .strip_prefix("set")
                .unwrap_or("")
                .trim_start()
                .to_lowercase();
            let opts = ["date-format", "week-start"];
            if filter.is_empty() {
                opts.iter().map(|s| s.to_string()).collect()
            } else {
                opts.iter()
                    .filter(|s| s.contains(filter.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            }
        } else if !trimmed.contains(' ') {
            // No space yet — complete command names.
            let prefix = trimmed.to_lowercase();
            [
                "config-open",
                "config-reload",
                "config-write",
                "quit",
                "set",
                "theme",
            ]
            .iter()
            .filter(|c| c.starts_with(prefix.as_str()))
            .map(|c| c.to_string())
            .collect()
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

        // Identify which prefix level we're completing.
        // Use the completions list as the signal: if "theme"/"set" appear as entries we're
        // still cycling top-level commands; if they don't, the buf prefix puts us in sub-mode.
        let is_theme_arg = matches!(&self.mode, AppMode::Command { buf, completions, .. } if {
            let t = buf.trim();
            (t == "theme" || t.starts_with("theme "))
                && !completions.contains(&"theme".to_string())
        });
        let is_set_date_format = matches!(&self.mode, AppMode::Command { buf, .. } if {
            let t = buf.trim();
            t == "set date-format" || t.starts_with("set date-format ")
        });
        let is_set_cmd = !is_set_date_format
            && matches!(&self.mode, AppMode::Command { buf, completions, .. } if {
                let t = buf.trim();
                (t == "set" || t.starts_with("set "))
                    && !completions.contains(&"set".to_string())
            });

        // Save original theme on the first tab in theme-arg mode.
        if is_theme_arg {
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
        }

        // Advance the selection index.
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
            if is_theme_arg {
                *buf = format!("theme {next_name}");
            } else if is_set_date_format {
                *buf = format!("set date-format {next_name}");
            } else if is_set_cmd {
                *buf = format!("set {next_name}");
            } else {
                *buf = next_name.clone();
            }
        }

        // Live-preview only applies to theme names.
        if is_theme_arg {
            let themes = self.themes.clone();
            self.theme = crate::theme::Theme::resolve(&next_name, &themes);
        }
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
                    self.theme_name = arg.to_string();
                    self.status = Some((
                        format!("Theme set to '{arg}'. Run :config-write to save."),
                        false,
                    ));
                }
            }
            "config-reload" => match tmkpr_lib::config::Config::load() {
                Ok(cfg) => {
                    let name = cfg.display.theme.clone();
                    self.themes = cfg.themes.clone();
                    let themes = self.themes.clone();
                    self.theme = crate::theme::Theme::resolve(&name, &themes);
                    self.theme_name = name;
                    self.date_format = cfg.display.date_format.clone();
                    self.week_start = chrono::Weekday::from(cfg.display.week_start);
                    self.status = Some(("Config reloaded.".into(), false));
                }
                Err(e) => {
                    self.status = Some((format!("Config reload failed: {e}"), true));
                }
            },
            "config-write" => match tmkpr_lib::config::Config::load() {
                Ok(mut cfg) => {
                    cfg.display.theme = self.theme_name.clone();
                    cfg.display.date_format = self.date_format.clone();
                    cfg.display.week_start = weekday_to_def(self.week_start);
                    if let Err(e) = cfg.save() {
                        self.status = Some((format!("Write failed: {e}"), true));
                    } else {
                        self.status = Some(("Config written.".into(), false));
                    }
                }
                Err(e) => {
                    self.status = Some((format!("Config write failed: {e}"), true));
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
            "set" => {
                let (setting, value) = match arg.split_once(' ') {
                    Some((s, v)) => (s, v.trim()),
                    None => (arg, ""),
                };
                match setting {
                    "date-format" => {
                        let chrono_fmt = DATE_FORMAT_PRESETS
                            .iter()
                            .find(|(display, _)| *display == value)
                            .map(|(_, fmt)| *fmt);
                        match chrono_fmt {
                            None => {
                                self.status = Some((
                                    "Use Tab to pick: YYYY-MM-DD HH:MM, DD-MM-YYYY HH:MM, MM-DD-YYYY HH:MM".into(),
                                    true,
                                ));
                            }
                            Some(fmt) => {
                                self.date_format = fmt.to_string();
                                self.status = Some((
                                    format!(
                                        "date-format set to '{value}'. Run :config-write to save."
                                    ),
                                    false,
                                ));
                            }
                        }
                    }
                    "week-start" => {
                        let weekday = match value.to_lowercase().as_str() {
                            "mon" | "monday" => Some(chrono::Weekday::Mon),
                            "tue" | "tuesday" => Some(chrono::Weekday::Tue),
                            "wed" | "wednesday" => Some(chrono::Weekday::Wed),
                            "thu" | "thursday" => Some(chrono::Weekday::Thu),
                            "fri" | "friday" => Some(chrono::Weekday::Fri),
                            "sat" | "saturday" => Some(chrono::Weekday::Sat),
                            "sun" | "sunday" => Some(chrono::Weekday::Sun),
                            _ => None,
                        };
                        match weekday {
                            None => {
                                self.status = Some((
                                    "Usage: set week-start <mon|tue|wed|thu|fri|sat|sun>".into(),
                                    true,
                                ));
                            }
                            Some(w) => {
                                self.week_start = w;
                                self.status = Some((
                                    format!(
                                        "week-start set to '{value}'. Run :config-write to save."
                                    ),
                                    false,
                                ));
                            }
                        }
                    }
                    _ => {
                        self.status = Some((
                            "Unknown setting. Available: date-format, week-start".into(),
                            true,
                        ));
                    }
                }
            }
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

    fn project_colors(&self) -> Vec<Option<String>> {
        self.projects.iter().map(|p| p.color.clone()).collect()
    }

    fn task_names(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter(|t| !t.completed)
            .map(|t| t.name.clone())
            .collect()
    }

    fn task_colors_all(&self) -> Vec<Option<String>> {
        self.tasks
            .iter()
            .filter(|t| !t.completed)
            .map(|t| {
                self.projects
                    .iter()
                    .find(|p| p.id == t.project_id)
                    .and_then(|p| p.color.clone())
            })
            .collect()
    }

    pub fn task_names_for_project(&self, project_name: &str) -> Vec<String> {
        let project = self.projects.iter().find(|p| p.name == project_name);
        if let Some(proj) = project {
            self.tasks
                .iter()
                .filter(|t| t.project_id == proj.id && !t.completed)
                .map(|t| t.name.clone())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn task_colors_for_project(&self, project_name: &str) -> Vec<Option<String>> {
        let project = self.projects.iter().find(|p| p.name == project_name);
        if let Some(proj) = project {
            let color = proj.color.clone();
            self.tasks
                .iter()
                .filter(|t| t.project_id == proj.id && !t.completed)
                .map(|_| color.clone())
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
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();
        self.mode = AppMode::StartModal(Form {
            fields: vec![
                Field::new("Project", "")
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::new("Task", "")
                    .with_completions(tasks)
                    .with_completion_colors(task_colors),
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
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();
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
                Field::new("Project", &project_val)
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::new("Task", &task_val)
                    .with_completions(tasks)
                    .with_completion_colors(task_colors),
                Field::new("Note", &note_val),
                Field::new("Tags (comma-separated)", ""),
            ],
            focused: 0,
        });
    }

    pub fn open_edit_modal(&mut self) {
        if self.entries[self.selected].is_event() {
            self.open_edit_event_modal();
            return;
        }
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
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();
        self.mode = AppMode::EditModal {
            id,
            form: Form {
                fields: vec![
                    Field::new("Project", project_val)
                        .with_completions(projects)
                        .with_completion_colors(project_colors),
                    Field::new("Task", task_val)
                        .with_completions(tasks)
                        .with_completion_colors(task_colors),
                    Field::new("Note", note_val),
                    Field::new("Start", start_val).into_timestamp(),
                    Field::new("End (blank = active)", end_val).into_timestamp(),
                    Field::new("Tags (comma-separated)", tags_val),
                ],
                focused: 0,
            },
        };
    }

    pub fn open_edit_event_modal(&mut self) {
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
        let time_val = entry
            .started_at
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let tags_val = entry.tags.join(", ");
        let projects = self.project_names();
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();
        self.mode = AppMode::EditEventModal {
            id,
            form: Form {
                fields: vec![
                    Field::new("Project", project_val)
                        .with_completions(projects)
                        .with_completion_colors(project_colors),
                    Field::new("Task", task_val)
                        .with_completions(tasks)
                        .with_completion_colors(task_colors),
                    Field::new("Note", note_val),
                    Field::new("Time (YYYY-MM-DD HH:MM)", time_val).into_timestamp(),
                    Field::new("Tags (comma-separated)", tags_val),
                ],
                focused: 0,
            },
        };
    }

    pub fn open_add_event_modal(&mut self) {
        let projects = self.project_names();
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();
        let now_val = Local::now().format("%Y-%m-%d %H:%M").to_string();
        self.mode = AppMode::AddEventModal(Form {
            fields: vec![
                Field::new("Project", "")
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::new("Task", "")
                    .with_completions(tasks)
                    .with_completion_colors(task_colors),
                Field::new("Note", ""),
                Field::new("Time (YYYY-MM-DD HH:MM)", now_val).into_timestamp(),
                Field::new("Tags (comma-separated)", ""),
            ],
            focused: 0,
        });
    }

    pub fn add_event_entry(
        &mut self,
        project: &str,
        task: &str,
        note: &str,
        time_str: &str,
        tags_str: &str,
    ) -> anyhow::Result<()> {
        let now = Utc::now();
        let at = if time_str.is_empty() {
            now
        } else {
            parse_datetime(time_str, now, TimeFormat::H24)
                .map_err(|e| anyhow::anyhow!("Invalid time '{}': {}", time_str, e))?
        };

        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let project_name = if project.is_empty() {
            None
        } else {
            Some(project)
        };
        let task_name = if task.is_empty() { None } else { Some(task) };
        let note_val = if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        };

        let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
        svc.log_event(project_name, task_name, note_val, tags, at)?;
        self.refresh()?;
        Ok(())
    }

    pub fn edit_event_entry(
        &mut self,
        id: &str,
        project: &str,
        task: &str,
        note: &str,
        time_str: &str,
        tags_str: &str,
    ) -> anyhow::Result<()> {
        self.edit_entry(id, project, task, note, time_str, time_str, tags_str)
    }

    pub fn open_add_manual_entry_modal(&mut self) {
        let projects = self.project_names();
        let project_colors = self.project_colors();
        let tasks = self.task_names();
        let task_colors = self.task_colors_all();

        let now = Local::now();
        let start_val = now.format("%Y-%m-%d %H:%M").to_string();

        self.mode = AppMode::AddManualEntry(Form {
            fields: vec![
                Field::new("Project", "")
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::new("Task", "")
                    .with_completions(tasks)
                    .with_completion_colors(task_colors),
                Field::new("Note", ""),
                Field::new("Start (YYYY-MM-DD HH:MM or HH:MM)", &start_val).into_timestamp(),
                Field::new("End (YYYY-MM-DD HH:MM or HH:MM)", "").into_timestamp(),
                Field::new("Tags (comma-separated)", ""),
                Field::toggle("Snap to existing activities", false),
            ],
            focused: 0,
        });
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
            let entry = svc.stop(None)?;

            // Retrieve project and task names for logging
            let project_name = entry
                .project_id
                .as_ref()
                .and_then(|pid| self.storage.get_project(pid).ok())
                .map(|p| p.name);
            let task_name = entry
                .task_id
                .as_ref()
                .and_then(|tid| self.storage.get_task(tid).ok())
                .map(|t| t.name);

            // Log to Obsidian if enabled
            let _ = obsidian_logger::log_activity_to_obsidian(
                &self.config,
                &entry,
                project_name.as_deref(),
                task_name.as_deref(),
                obsidian_logger::ActivityAction::Stopped,
            );
        }
        self.refresh()?;
        self.status = Some(("Stopped.".into(), false));
        Ok(())
    }

    pub fn delete_entry(&mut self, id: &str) -> anyhow::Result<()> {
        // Log to Obsidian if enabled
        if let Ok(entry) = self.storage.get_entry(id) {
            let project_name = entry
                .project_id
                .as_ref()
                .and_then(|pid| self.storage.get_project(pid).ok())
                .map(|p| p.name);
            let task_name = entry
                .task_id
                .as_ref()
                .and_then(|tid| self.storage.get_task(tid).ok())
                .map(|t| t.name);

            let _ = obsidian_logger::log_activity_to_obsidian(
                &self.config,
                &entry,
                project_name.as_deref(),
                task_name.as_deref(),
                obsidian_logger::ActivityAction::Deleted,
            );
        }

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
        let entry = {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.start(project_opt, task_opt, note_opt, tags, None)?
        };
        // Log to Obsidian if enabled
        let _ = obsidian_logger::log_activity_to_obsidian(
            &self.config,
            &entry,
            project_opt,
            task_opt,
            obsidian_logger::ActivityAction::Started,
        );
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

    fn find_nearby_activities(
        &self,
        time: chrono::DateTime<Utc>,
        window_minutes: i64,
    ) -> Vec<(&Entry, &'static str)> {
        let window = Duration::minutes(window_minutes);
        let start = time - window;
        let end = time + window;

        let mut nearby = Vec::new();
        for entry in &self.entries {
            if entry.started_at >= start && entry.started_at <= end {
                nearby.push((entry, "start"));
            }
            if let Some(finished) = entry.finished_at {
                if finished >= start && finished <= end {
                    nearby.push((entry, "end"));
                }
            }
        }
        nearby.sort_by_key(|(e, kind)| {
            let time_val = if *kind == "start" {
                e.started_at
            } else {
                e.finished_at.unwrap_or(e.started_at)
            };
            (time_val - time).abs()
        });
        nearby
    }

    fn snap_time_to_activity(
        &mut self,
        time_str: &str,
        snap_enabled: bool,
    ) -> anyhow::Result<String> {
        if !snap_enabled || time_str.is_empty() {
            return Ok(time_str.to_string());
        }

        let now = Utc::now();
        let parsed_time = parse_datetime(time_str, now, TimeFormat::H24)?;
        let nearby = self.find_nearby_activities(parsed_time, 60);

        if let Some((entry, kind)) = nearby.first() {
            let snapped_time = if *kind == "start" {
                entry.started_at
            } else {
                entry.finished_at.unwrap_or(entry.started_at)
            };
            let project_name = entry
                .project_id
                .as_ref()
                .and_then(|pid| self.storage.get_project(pid).ok())
                .map(|p| p.name)
                .unwrap_or_else(|| "Unnamed".to_string());

            let snapped_str = snapped_time
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string();
            self.status = Some((
                format!("Snapped to {} ({})", project_name, snapped_str),
                false,
            ));
            Ok(snapped_str)
        } else {
            Ok(time_str.to_string())
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_manual_entry(
        &mut self,
        project: &str,
        task: &str,
        note: &str,
        start_str: &str,
        end_str: &str,
        tags_str: &str,
        snap_to_existing: bool,
    ) -> anyhow::Result<()> {
        if start_str.is_empty() {
            return Err(anyhow::anyhow!("Start time is required"));
        }
        if end_str.is_empty() {
            return Err(anyhow::anyhow!("End time is required"));
        }

        let now = Utc::now();

        let snapped_start = self.snap_time_to_activity(start_str, snap_to_existing)?;
        let snapped_end = self.snap_time_to_activity(end_str, snap_to_existing)?;

        let started_at = parse_datetime(&snapped_start, now, TimeFormat::H24)
            .map_err(|e| anyhow::anyhow!("Invalid start time: {}", e))?;

        let finished_at = parse_datetime(&snapped_end, now, TimeFormat::H24)
            .map_err(|e| anyhow::anyhow!("Invalid end time: {}", e))?;

        if started_at >= finished_at {
            return Err(anyhow::anyhow!("Start time must be before end time"));
        }

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

        let entry = {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.log(
                project_opt,
                task_opt,
                note_opt,
                tags,
                started_at,
                finished_at,
            )?
        };

        let _ = obsidian_logger::log_activity_to_obsidian(
            &self.config,
            &entry,
            project_opt,
            task_opt,
            obsidian_logger::ActivityAction::Merged,
        );

        self.refresh()?;
        self.status = Some(("Entry created.".into(), false));
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
        let merged = svc.merge_into_next(&id)?;

        // Log to Obsidian if enabled
        let project_name = merged
            .project_id
            .as_ref()
            .and_then(|pid| self.storage.get_project(pid).ok())
            .map(|p| p.name);
        let task_name = merged
            .task_id
            .as_ref()
            .and_then(|tid| self.storage.get_task(tid).ok())
            .map(|t| t.name);

        let _ = obsidian_logger::log_activity_to_obsidian(
            &self.config,
            &merged,
            project_name.as_deref(),
            task_name.as_deref(),
            obsidian_logger::ActivityAction::Merged,
        );

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
            let comment = svc.add(Some(&entry_id), body)?;
            // Log to Obsidian if enabled
            let _ = obsidian_logger::log_comment_to_obsidian(&self.config, &comment);
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

        let tasks = self.storage.list_tasks(&id, false)?;
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
        // Log to Obsidian if enabled
        let _ = obsidian_logger::log_project_to_obsidian(
            &self.config,
            name,
            obsidian_logger::ProjectAction::Deleted,
        );

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
            fields: vec![Field::toggle(
                "Show archived projects",
                !self.project_filter.hide_archived,
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
        let project_colors = self.project_colors();
        self.mode = AppMode::AddTask(Form {
            fields: vec![
                Field::new("Project", "")
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
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

        // Log to Obsidian if enabled
        if let Ok(project) = self.storage.get_project(&project_id) {
            let _ = obsidian_logger::log_project_to_obsidian(
                &self.config,
                &project.name,
                obsidian_logger::ProjectAction::Updated,
            );
        }

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
        let project_colors = self.project_colors();
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
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::toggle("Show archived tasks", !self.task_filter.hide_archived),
                Field::toggle("Show completed tasks", !self.task_filter.hide_completed),
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

        // Log to Obsidian if enabled
        if let Ok(task) = self.storage.get_task(&task_id) {
            let project_name = self
                .storage
                .get_project(&task.project_id)
                .ok()
                .map(|p| p.name)
                .unwrap_or_default();

            let _ = obsidian_logger::log_task_to_obsidian(
                &self.config,
                &project_name,
                &task.name,
                obsidian_logger::TaskAction::Updated,
            );
        }

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

        // Log to Obsidian if enabled
        if let Ok(task) = self.storage.get_task(&task_id) {
            let project_name = self
                .storage
                .get_project(&task.project_id)
                .ok()
                .map(|p| p.name)
                .unwrap_or_default();

            let action = if !currently_completed {
                obsidian_logger::TaskAction::Completed
            } else {
                obsidian_logger::TaskAction::Updated
            };

            let _ = obsidian_logger::log_task_to_obsidian(
                &self.config,
                &project_name,
                &task.name,
                action,
            );
        }

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
        let (project_name, task_name, task_id) =
            if let AppMode::ManageTasks { tasks, selected } = &self.mode {
                if *selected < tasks.len() {
                    let task = &tasks[*selected];
                    (
                        self.project_name(&task.project_id).to_string(),
                        task.name.clone(),
                        task.id.clone(),
                    )
                } else {
                    return Ok(());
                }
            } else {
                return Ok(());
            };

        // Log to Obsidian if enabled
        if let Ok(task) = self.storage.get_task(&task_id) {
            let project_name_str = self.project_name(&task.project_id);

            let _ = obsidian_logger::log_task_to_obsidian(
                &self.config,
                project_name_str,
                &task.name,
                obsidian_logger::TaskAction::Deleted,
            );
        }

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
        let project_colors = self.project_colors();
        self.mode = AppMode::Filter(Form {
            fields: vec![
                Field::new("Project (empty = all)", &self.entry_filter.project_name)
                    .with_completions(projects)
                    .with_completion_colors(project_colors),
                Field::new(
                    "Date: today/yesterday/this week/YYYY-MM-DD or YYYY-MM-DD..YYYY-MM-DD",
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

        let (from, until) = parse_date_filter(date_str, self.week_start)?;
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
                // Silently clear the project filter if the project no longer exists in
                // this database (e.g. the state was saved with a different TMKPR_DB).
                let project = if !state.entry_filter_project.is_empty() {
                    let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
                    match svc.get_by_name(&state.entry_filter_project)? {
                        Some(_) => state.entry_filter_project.clone(),
                        None => String::new(),
                    }
                } else {
                    state.entry_filter_project.clone()
                };
                self.apply_filter(&project, &state.entry_filter_date)?;
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

    pub fn prev_week(&mut self) -> anyhow::Result<()> {
        let iso_date = NaiveDate::from_isoywd_opt(
            self.displayed_week_year,
            self.displayed_week_num,
            Weekday::Mon,
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid week"))?;
        let prev_iso_date = iso_date - Duration::days(7);
        let prev_iso_week = prev_iso_date.iso_week();
        self.displayed_week_year = prev_iso_week.year();
        self.displayed_week_num = prev_iso_week.week();
        self.refresh_week_report()?;
        Ok(())
    }

    pub fn next_week(&mut self) -> anyhow::Result<()> {
        let iso_date = NaiveDate::from_isoywd_opt(
            self.displayed_week_year,
            self.displayed_week_num,
            Weekday::Mon,
        )
        .ok_or_else(|| anyhow::anyhow!("Invalid week"))?;
        let next_iso_date = iso_date + Duration::days(7);
        let next_iso_week = next_iso_date.iso_week();
        self.displayed_week_year = next_iso_week.year();
        self.displayed_week_num = next_iso_week.week();
        self.refresh_week_report()?;
        Ok(())
    }

    fn refresh_week_report(&mut self) -> anyhow::Result<()> {
        let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
        self.week_report = svc
            .week_report(self.displayed_week_year, self.displayed_week_num, false)
            .ok();
        Ok(())
    }

    pub fn open_settings(&mut self) {
        let mut theme_names: Vec<String> = crate::theme::Theme::builtin_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for name in self.themes.keys() {
            if !theme_names.contains(name) {
                theme_names.push(name.clone());
            }
        }
        theme_names.sort();

        let theme_idx = theme_names
            .iter()
            .position(|n| n == &self.theme_name)
            .unwrap_or(0);

        let date_fmt_idx = DATE_FORMAT_PRESETS
            .iter()
            .position(|(_, fmt)| *fmt == self.date_format)
            .unwrap_or(0);

        let obs = &self.config.obsidian;
        let obs_vault = obs
            .vault_dir
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let obs_activity = obs.activity_category.clone().unwrap_or_default();
        let obs_comment = obs.comment_category.clone().unwrap_or_default();

        self.mode = AppMode::Settings {
            cursor: 0,
            theme_names,
            theme_idx,
            date_fmt_idx,
            week_start: self.week_start,
            obs_enabled: self.config.obsidian.enabled,
            obs_vault,
            obs_activity,
            obs_comment,
            text_editing: false,
        };
    }

    pub fn settings_save(&mut self) -> anyhow::Result<()> {
        let AppMode::Settings {
            theme_names,
            theme_idx,
            date_fmt_idx,
            week_start,
            obs_enabled,
            obs_vault,
            obs_activity,
            obs_comment,
            ..
        } = &self.mode
        else {
            return Ok(());
        };

        let theme_name = theme_names.get(*theme_idx).cloned().unwrap_or_default();
        let date_format = DATE_FORMAT_PRESETS
            .get(*date_fmt_idx)
            .map(|(_, fmt)| fmt.to_string())
            .unwrap_or_else(|| self.date_format.clone());
        let week_start = *week_start;
        let obs_enabled = *obs_enabled;
        let obs_vault = obs_vault.clone();
        let obs_activity = obs_activity.clone();
        let obs_comment = obs_comment.clone();

        // Apply to live state
        let themes = self.themes.clone();
        self.theme = crate::theme::Theme::resolve(&theme_name, &themes);
        self.theme_name = theme_name.clone();
        self.date_format = date_format.clone();
        self.week_start = week_start;

        // Apply to config struct and write to disk
        self.config.display.theme = theme_name;
        self.config.display.date_format = date_format;
        self.config.display.week_start = weekday_to_def(week_start);
        self.config.obsidian.enabled = obs_enabled;
        self.config.obsidian.vault_dir = if obs_vault.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(&obs_vault))
        };
        self.config.obsidian.activity_category = if obs_activity.is_empty() {
            None
        } else {
            Some(obs_activity)
        };
        self.config.obsidian.comment_category = if obs_comment.is_empty() {
            None
        } else {
            Some(obs_comment)
        };

        self.config.save()?;
        self.mode = AppMode::Normal;
        self.status = Some(("Settings saved.".into(), false));
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
            "default".to_string(),
            crate::theme::Theme::from_name("default"),
            HashMap::new(),
            "%H:%M".to_string(),
            chrono::Weekday::Mon,
            Config::default(),
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
        let (from, until) = parse_date_filter("", chrono::Weekday::Mon).unwrap();
        assert!(from.is_none());
        assert!(until.is_none());
    }

    #[test]
    fn parse_date_filter_whitespace_returns_none() {
        let (from, until) = parse_date_filter("   ", chrono::Weekday::Mon).unwrap();
        assert!(from.is_none());
        assert!(until.is_none());
    }

    #[test]
    fn parse_date_filter_today() {
        let (from, until) = parse_date_filter("today", chrono::Weekday::Mon).unwrap();
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
        let (from, until) = parse_date_filter("yesterday", chrono::Weekday::Mon).unwrap();
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
        let (from, until) = parse_date_filter("this week", chrono::Weekday::Mon).unwrap();
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
        let (from, until) = parse_date_filter("2024-05-15", chrono::Weekday::Mon).unwrap();
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
        let (from, until) =
            parse_date_filter("2024-05-15..2024-05-18", chrono::Weekday::Mon).unwrap();
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
        let result = parse_date_filter("invalid-date", chrono::Weekday::Mon);
        assert!(result.is_err());
    }

    #[test]
    fn parse_date_filter_slash_separator_error() {
        let result = parse_date_filter("2024-05-15/2024-05-18", chrono::Weekday::Mon);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Date range uses '..' not '/'"));
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

    // --- open_settings ---

    #[test]
    fn open_settings_selects_correct_theme_idx() {
        let mut app = make_app();
        app.theme_name = "dracula".to_string();
        app.open_settings();
        let AppMode::Settings {
            theme_names,
            theme_idx,
            ..
        } = &app.mode
        else {
            panic!("expected Settings mode");
        };
        assert_eq!(theme_names[*theme_idx], "dracula");
    }

    #[test]
    fn open_settings_selects_correct_date_fmt_idx() {
        let mut app = make_app();
        app.date_format = "%d-%m-%Y %H:%M".to_string();
        app.open_settings();
        let AppMode::Settings { date_fmt_idx, .. } = &app.mode else {
            panic!("expected Settings mode");
        };
        let (_, fmt) = DATE_FORMAT_PRESETS[*date_fmt_idx];
        assert_eq!(fmt, "%d-%m-%Y %H:%M");
    }

    #[test]
    fn open_settings_unknown_date_fmt_defaults_to_zero() {
        let mut app = make_app();
        app.date_format = "%X".to_string();
        app.open_settings();
        let AppMode::Settings { date_fmt_idx, .. } = &app.mode else {
            panic!("expected Settings mode");
        };
        assert_eq!(*date_fmt_idx, 0);
    }

    #[test]
    fn open_settings_reads_week_start() {
        let mut app = make_app();
        app.week_start = chrono::Weekday::Sun;
        app.open_settings();
        let AppMode::Settings { week_start, .. } = &app.mode else {
            panic!("expected Settings mode");
        };
        assert_eq!(*week_start, chrono::Weekday::Sun);
    }

    #[test]
    fn open_settings_reads_obsidian_config() {
        let mut app = make_app();
        app.config.obsidian.enabled = true;
        app.config.obsidian.vault_dir = Some(std::path::PathBuf::from("/my/vault"));
        app.config.obsidian.activity_category = Some("Work".to_string());
        app.config.obsidian.comment_category = Some("Notes".to_string());
        app.open_settings();
        let AppMode::Settings {
            obs_enabled,
            obs_vault,
            obs_activity,
            obs_comment,
            ..
        } = &app.mode
        else {
            panic!("expected Settings mode");
        };
        assert!(*obs_enabled);
        assert_eq!(obs_vault, "/my/vault");
        assert_eq!(obs_activity, "Work");
        assert_eq!(obs_comment, "Notes");
    }

    // --- settings_save ---

    fn app_in_settings(theme: &str, date_fmt_idx: usize, week_start: chrono::Weekday) -> App {
        let mut app = make_app();
        app.open_settings();
        // Override the draft values without going through input handlers
        if let AppMode::Settings {
            theme_names,
            theme_idx,
            date_fmt_idx: di,
            week_start: ws,
            ..
        } = &mut app.mode
        {
            *theme_idx = theme_names.iter().position(|n| n == theme).unwrap_or(0);
            *di = date_fmt_idx;
            *ws = week_start;
        }
        app
    }

    #[test]
    fn settings_save_applies_date_format_to_live_state() {
        let mut app = app_in_settings("default", 1, chrono::Weekday::Mon);
        app.settings_save().unwrap();
        assert_eq!(app.date_format, DATE_FORMAT_PRESETS[1].1);
    }

    #[test]
    fn settings_save_applies_week_start_to_live_state() {
        let mut app = app_in_settings("default", 0, chrono::Weekday::Wed);
        app.settings_save().unwrap();
        assert_eq!(app.week_start, chrono::Weekday::Wed);
    }

    #[test]
    fn settings_save_applies_theme_name() {
        let mut app = app_in_settings("dracula", 0, chrono::Weekday::Mon);
        app.settings_save().unwrap();
        assert_eq!(app.theme_name, "dracula");
    }

    #[test]
    fn settings_save_updates_config_struct() {
        let mut app = app_in_settings("default", 2, chrono::Weekday::Fri);
        app.settings_save().unwrap();
        assert_eq!(app.config.display.date_format, DATE_FORMAT_PRESETS[2].1);
    }

    #[test]
    fn settings_save_empty_obsidian_vault_becomes_none() {
        let mut app = make_app();
        app.open_settings();
        if let AppMode::Settings { obs_vault, .. } = &mut app.mode {
            *obs_vault = String::new();
        }
        app.settings_save().unwrap();
        assert!(app.config.obsidian.vault_dir.is_none());
    }

    #[test]
    fn settings_save_nonempty_obsidian_fields_set() {
        let mut app = make_app();
        app.open_settings();
        if let AppMode::Settings {
            obs_enabled,
            obs_vault,
            obs_activity,
            obs_comment,
            ..
        } = &mut app.mode
        {
            *obs_enabled = true;
            *obs_vault = "/vault".to_string();
            *obs_activity = "Activity".to_string();
            *obs_comment = "Comment".to_string();
        }
        app.settings_save().unwrap();
        assert!(app.config.obsidian.enabled);
        assert_eq!(
            app.config.obsidian.vault_dir,
            Some(std::path::PathBuf::from("/vault"))
        );
        assert_eq!(
            app.config.obsidian.activity_category.as_deref(),
            Some("Activity")
        );
        assert_eq!(
            app.config.obsidian.comment_category.as_deref(),
            Some("Comment")
        );
    }

    #[test]
    fn settings_save_sets_mode_to_normal() {
        let mut app = app_in_settings("default", 0, chrono::Weekday::Mon);
        app.settings_save().unwrap();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn settings_save_noop_when_not_in_settings_mode() {
        let mut app = make_app();
        // mode is Normal, not Settings
        let result = app.settings_save();
        assert!(result.is_ok());
        assert!(matches!(app.mode, AppMode::Normal));
    }
}
