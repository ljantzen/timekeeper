use chrono::{Datelike, Local, Utc};
use ratatui::widgets::ListState;
use tmkpr_lib::{
    models::{
        entry::{Entry, EntryFilter, UpdateEntry},
        project::Project,
        task::Task,
    },
    nlp::parser::{parse_datetime, TimeFormat},
    service::{EntryService, ProjectService, TaskService, WeekReport},
    storage::Storage,
};

use crate::form::{Field, Form};

pub enum AppMode {
    Normal,
    StartModal(Form),
    EditModal { id: String, form: Form },
    ConfirmDelete { id: String, display: String },
    AddProject(Form),
    AddTask(Form),
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
    Help,
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
        }
    }

    pub fn mode_kind(&self) -> ModeKind {
        match &self.mode {
            AppMode::Normal => ModeKind::Normal,
            AppMode::StartModal(_) => ModeKind::StartModal,
            AppMode::EditModal { .. } => ModeKind::EditModal,
            AppMode::ConfirmDelete { .. } => ModeKind::ConfirmDelete,
            AppMode::AddProject(_) => ModeKind::AddProject,
            AppMode::AddTask(_) => ModeKind::AddTask,
            AppMode::Help => ModeKind::Help,
        }
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        {
            let svc = ProjectService::new(self.storage.as_ref(), &self.user_id);
            self.projects = svc.list(false)?;
        }
        {
            let mut tasks = vec![];
            for project in &self.projects {
                let pt = self.storage.list_tasks(&project.id, false).unwrap_or_default();
                tasks.extend(pt);
            }
            self.tasks = tasks;
        }
        self.active_entry = self.storage.get_active_entry(&self.user_id)?;
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            self.entries = svc.list(EntryFilter {
                user_id: self.user_id.clone(),
                limit: Some(50),
                include_active: false,
                ..Default::default()
            })?;
        }
        {
            let now = Local::now();
            let iso = now.iso_week();
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            self.week_report = svc.week_report(iso.year(), iso.week()).ok();
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
            ],
            focused: 0,
        });
    }

    pub fn open_edit_modal(&mut self) {
        if self.entries.is_empty() {
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
            .map(|t| {
                t.with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default();

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
                ],
                focused: 0,
            },
        };
    }

    pub fn open_confirm_delete(&mut self) {
        if self.entries.is_empty() {
            return;
        }
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

    pub fn start_entry(&mut self, project: &str, task: &str, note: &str) -> anyhow::Result<()> {
        let project_opt = if project.is_empty() { None } else { Some(project) };
        let task_opt = if task.is_empty() { None } else { Some(task) };
        let note_opt = if note.is_empty() {
            None
        } else {
            Some(note.to_string())
        };
        {
            let svc = EntryService::new(self.storage.as_ref(), &self.user_id);
            svc.start(project_opt, task_opt, note_opt, vec![], None)?;
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
                    ..Default::default()
                },
            )?;
        }
        self.refresh()?;
        self.status = Some(("Updated.".into(), false));
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

    pub fn add_project(&mut self, name: &str, description: &str, color: &str) -> anyhow::Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Project name is required"));
        }
        let desc = if description.is_empty() { None } else { Some(description.to_string()) };
        let col = if color.is_empty() { None } else { Some(color.to_string()) };
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
        let desc = if description.is_empty() { None } else { Some(description.to_string()) };
        {
            let svc = TaskService::new(self.storage.as_ref(), &self.user_id);
            svc.add(project, name, desc)?;
        }
        self.refresh()?;
        self.status = Some((format!("Task '{name}' created in '{project}'."), false));
        Ok(())
    }
}
