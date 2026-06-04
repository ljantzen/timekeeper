use anyhow::Result;
use std::time::{Duration, Instant};
use tmkpr_lib::{
    config::{Config, PomodoroConfig},
    models::project::Project,
    models::task::{NewTask, Task},
    obsidian_logger,
    service::EntryService,
    storage::Storage,
};

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundField {
    WorkToBreak,
    BreakToWork,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Screen {
    Main,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditMode {
    None,
    EditProject,
    EditTask,
    ConfirmDelete,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimerState {
    Stopped,
    Running,
    Paused,
    Break,
}

pub struct App<'a> {
    storage: &'a dyn Storage,
    user_id: &'a str,
    projects: Vec<Project>,
    selected_project_idx: usize,
    tasks: Vec<Task>,
    selected_task_idx: usize,
    timer_state: TimerState,
    elapsed: Duration,
    session_start: Option<Instant>,
    paused_at: Option<Instant>,
    message: Option<String>,
    work_duration: Duration,
    break_duration: Duration,
    long_break_duration: Duration,
    sessions_before_long_break: u64,
    work_sessions_completed: u64,
    notify_desktop: bool,
    sound_work_to_break: Option<String>,
    sound_break_to_work: Option<String>,
    message_timeout: Duration,
    message_set_at: Option<Instant>,
    auto_start_break: bool,
    max_cycles: u64,
    screen: Screen,
    config: Config,
    settings_edit: PomodoroConfig,
    settings_cursor: usize,
    sound_editing: Option<SoundField>,
    sound_edit_buf: String,
    new_task_editing: bool,
    new_task_buf: String,
    new_project_editing: bool,
    new_project_buf: String,
    edit_mode: EditMode,
    edit_buf: String,
    delete_target: Option<&'static str>,
    hide_completed_tasks: bool,
    task_queue: Vec<Task>,
    selected_queue_idx: usize,
    theme: Theme,
}

impl<'a> App<'a> {
    pub fn new(
        storage: &'a dyn Storage,
        user_id: &'a str,
        config: Config,
        theme: Theme,
    ) -> Result<Self> {
        let projects = storage.list_projects(user_id, false).unwrap_or_default();
        let tasks = if !projects.is_empty() {
            storage
                .list_tasks(&projects[0].id, false)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let hide_completed_tasks = config.pomodoro.hide_completed_tasks;

        Ok(Self {
            storage,
            user_id,
            projects,
            selected_project_idx: 0,
            tasks,
            selected_task_idx: 0,
            timer_state: TimerState::Stopped,
            elapsed: Duration::ZERO,
            session_start: None,
            paused_at: None,
            message: None,
            work_duration: Duration::from_secs(config.pomodoro.work_duration_minutes * 60),
            break_duration: Duration::from_secs(config.pomodoro.break_duration_minutes * 60),
            long_break_duration: Duration::from_secs(
                config.pomodoro.long_break_duration_minutes * 60,
            ),
            sessions_before_long_break: config.pomodoro.sessions_before_long_break,
            work_sessions_completed: 0,
            notify_desktop: config.pomodoro.notify_desktop,
            sound_work_to_break: config.pomodoro.sound_work_to_break.clone(),
            sound_break_to_work: config.pomodoro.sound_break_to_work.clone(),
            message_timeout: Duration::from_secs(config.pomodoro.message_timeout_secs),
            message_set_at: None,
            auto_start_break: config.pomodoro.auto_start_break,
            max_cycles: config.pomodoro.max_cycles,
            screen: Screen::Main,
            settings_edit: config.pomodoro.clone(),
            config,
            settings_cursor: 0,
            sound_editing: None,
            sound_edit_buf: String::new(),
            new_task_editing: false,
            new_task_buf: String::new(),
            new_project_editing: false,
            new_project_buf: String::new(),
            edit_mode: EditMode::None,
            edit_buf: String::new(),
            delete_target: None,
            hide_completed_tasks,
            task_queue: Vec::new(),
            selected_queue_idx: 0,
            theme,
        })
    }

    pub fn next_project(&mut self) {
        if self.timer_state == TimerState::Stopped {
            self.selected_project_idx =
                (self.selected_project_idx + 1) % self.projects.len().max(1);
            self.refresh_tasks();
        }
    }

    pub fn previous_project(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.projects.is_empty() {
            self.selected_project_idx = if self.selected_project_idx == 0 {
                self.projects.len() - 1
            } else {
                self.selected_project_idx - 1
            };
            self.refresh_tasks();
        }
    }

    pub fn next_task(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.tasks.is_empty() {
            self.selected_task_idx = (self.selected_task_idx + 1) % self.tasks.len();
        }
    }

    pub fn previous_task(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.tasks.is_empty() {
            self.selected_task_idx = if self.selected_task_idx == 0 {
                self.tasks.len() - 1
            } else {
                self.selected_task_idx - 1
            };
        }
    }

    pub fn start_timer(&mut self) -> Result<()> {
        if self.timer_state == TimerState::Stopped {
            if self.selected_task().map(|t| t.completed).unwrap_or(false) {
                self.message = Some("Task is completed. Reactivate it first.".to_string());
                return Ok(());
            }
            self.timer_state = TimerState::Running;
            self.session_start = Some(Instant::now());
            self.elapsed = Duration::ZERO;
            self.message = Some("Timer started! Press space to pause.".to_string());

            let svc = EntryService::new(self.storage, self.user_id);
            if let Some(proj) = self.selected_project() {
                if let Some(task) = self.selected_task() {
                    if let Ok(entry) = svc.start(Some(&proj.name), Some(&task.name), None, vec![], None) {
                        // Log to Obsidian if enabled
                        let _ = obsidian_logger::log_activity_to_obsidian(
                            &self.config,
                            &entry,
                            Some(&proj.name),
                            Some(&task.name),
                            obsidian_logger::ActivityAction::Started,
                        );
                    }
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn start_break(&mut self) -> Result<()> {
        if self.timer_state == TimerState::Stopped {
            self.work_sessions_completed += 1;
            let is_long_break = self
                .work_sessions_completed
                .is_multiple_of(self.sessions_before_long_break);

            if is_long_break {
                let svc = EntryService::new(self.storage, self.user_id);
                if let Ok(entry) = svc.stop(None) {
                    let proj = self.selected_project().map(|p| p.name.as_str());
                    let task = self.selected_task().map(|t| t.name.as_str());
                    let _ = obsidian_logger::log_activity_to_obsidian(
                        &self.config,
                        &entry,
                        proj,
                        task,
                        obsidian_logger::ActivityAction::Stopped,
                    );
                }
            }

            self.timer_state = TimerState::Break;
            self.session_start = Some(Instant::now());
            self.elapsed = Duration::ZERO;
            let break_msg = if is_long_break {
                "Break started! (Long break)"
            } else {
                "Break started!"
            };
            self.message = Some(break_msg.to_string());
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn toggle_timer(&mut self) {
        match self.timer_state {
            TimerState::Running => {
                self.timer_state = TimerState::Paused;
                self.paused_at = Some(Instant::now());
                self.message = Some("Timer paused. Press space to resume.".to_string());
            }
            TimerState::Paused => {
                if let Some(_paused) = self.paused_at {
                    self.session_start = Some(Instant::now() - self.elapsed);
                }
                self.timer_state = TimerState::Running;
                self.paused_at = None;
                self.message = Some("Timer resumed.".to_string());
            }
            _ => {}
        }
    }

    pub fn log_session(&mut self) -> Result<()> {
        if self.timer_state != TimerState::Stopped && self.elapsed > Duration::ZERO {
            let _elapsed = self.elapsed;
            let proj = self
                .selected_project()
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let task = self
                .selected_task()
                .map(|t| t.name.clone())
                .unwrap_or_default();
            let _color = self.selected_project().and_then(|p| p.color.clone());

            let svc = EntryService::new(self.storage, self.user_id);
            let entry = svc.stop(None)?;
            // Log to Obsidian if enabled
            let proj_name = if proj.is_empty() { None } else { Some(proj.as_str()) };
            let task_name = if task.is_empty() { None } else { Some(task.as_str()) };
            let _ = obsidian_logger::log_activity_to_obsidian(
                &self.config,
                &entry,
                proj_name,
                task_name,
                obsidian_logger::ActivityAction::Stopped,
            );

            self.message = Some("Session logged!".to_string());
            self.reset();
            Ok(())
        } else {
            self.message = Some("No active session to log.".to_string());
            Ok(())
        }
    }

    pub fn reset(&mut self) {
        let svc = EntryService::new(self.storage, self.user_id);
        if let Ok(entry) = svc.stop(None) {
            let proj = self.selected_project().map(|p| p.name.as_str());
            let task = self.selected_task().map(|t| t.name.as_str());
            let _ = obsidian_logger::log_activity_to_obsidian(
                &self.config,
                &entry,
                proj,
                task,
                obsidian_logger::ActivityAction::Stopped,
            );
        }

        self.timer_state = TimerState::Stopped;
        self.elapsed = Duration::ZERO;
        self.session_start = None;
        self.paused_at = None;
    }

    fn notify(&mut self, title: &str, body: &str, sound: Option<&str>) {
        self.message = Some(body.to_string());
        self.message_set_at = Some(Instant::now());

        if let Some(path) = sound {
            play_sound(path);
        }

        if self.notify_desktop {
            let _ = notify_rust::Notification::new()
                .summary(title)
                .body(body)
                .show();
        }
    }
}

fn play_sound(path: &str) {
    let path = path.to_string();
    std::thread::spawn(move || {
        let Ok(handle) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return;
        };
        let Ok(file) = std::fs::File::open(&path) else {
            return;
        };
        let Ok(player) = rodio::play(handle.mixer(), std::io::BufReader::new(file)) else {
            return;
        };
        player.sleep_until_end();
    });
}

impl<'a> App<'a> {
    pub fn open_settings(&mut self) {
        if self.timer_state == TimerState::Stopped {
            self.settings_edit = self.config.pomodoro.clone();
            self.settings_cursor = 0;
            self.sound_editing = None;
            self.sound_edit_buf = String::new();
            self.screen = Screen::Settings;
        }
    }

    pub fn settings_cursor_up(&mut self) {
        if self.settings_cursor == 0 {
            self.settings_cursor = 9;
        } else {
            self.settings_cursor -= 1;
        }
    }

    pub fn settings_cursor_down(&mut self) {
        if self.settings_cursor == 9 {
            self.settings_cursor = 0;
        } else {
            self.settings_cursor += 1;
        }
    }

    pub fn settings_adjust(&mut self, delta: i64) {
        match self.settings_cursor {
            0 => {
                self.settings_edit.work_duration_minutes =
                    (self.settings_edit.work_duration_minutes as i64 + delta).max(1) as u64;
            }
            1 => {
                self.settings_edit.break_duration_minutes =
                    (self.settings_edit.break_duration_minutes as i64 + delta).max(1) as u64;
            }
            2 => {
                self.settings_edit.sessions_before_long_break =
                    (self.settings_edit.sessions_before_long_break as i64 + delta).max(1) as u64;
            }
            3 => {
                self.settings_edit.long_break_duration_minutes =
                    (self.settings_edit.long_break_duration_minutes as i64 + delta).max(1) as u64;
            }
            4 => {
                self.settings_edit.max_cycles =
                    (self.settings_edit.max_cycles as i64 + delta).max(0) as u64;
            }
            5 if delta != 0 => {
                self.settings_edit.notify_desktop = !self.settings_edit.notify_desktop;
            }
            6 => {
                self.settings_edit.message_timeout_secs =
                    (self.settings_edit.message_timeout_secs as i64 + delta).max(0) as u64;
            }
            7 if delta != 0 => {
                self.settings_edit.auto_start_break = !self.settings_edit.auto_start_break;
            }
            _ => {}
        }
    }

    pub fn settings_save(&mut self) -> Result<()> {
        self.config.pomodoro = self.settings_edit.clone();
        self.config.save()?;

        self.work_duration = Duration::from_secs(self.config.pomodoro.work_duration_minutes * 60);
        self.break_duration = Duration::from_secs(self.config.pomodoro.break_duration_minutes * 60);
        self.long_break_duration =
            Duration::from_secs(self.config.pomodoro.long_break_duration_minutes * 60);
        self.sessions_before_long_break = self.config.pomodoro.sessions_before_long_break;
        self.notify_desktop = self.config.pomodoro.notify_desktop;
        self.message_timeout = Duration::from_secs(self.config.pomodoro.message_timeout_secs);
        self.auto_start_break = self.config.pomodoro.auto_start_break;
        self.max_cycles = self.config.pomodoro.max_cycles;
        self.sound_work_to_break = self.config.pomodoro.sound_work_to_break.clone();
        self.sound_break_to_work = self.config.pomodoro.sound_break_to_work.clone();

        self.sound_editing = None;
        self.sound_edit_buf = String::new();
        self.screen = Screen::Main;
        Ok(())
    }

    pub fn settings_cancel(&mut self) {
        self.sound_editing = None;
        self.sound_edit_buf = String::new();
        self.screen = Screen::Main;
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn settings_state(&self) -> (&PomodoroConfig, usize) {
        (&self.settings_edit, self.settings_cursor)
    }

    pub fn settings_cursor_on_sound_field(&self) -> bool {
        self.settings_cursor >= 8
    }

    pub fn is_editing_sound(&self) -> bool {
        self.sound_editing.is_some()
    }

    pub fn sound_editing(&self) -> Option<SoundField> {
        self.sound_editing
    }

    pub fn sound_edit_buf(&self) -> &str {
        &self.sound_edit_buf
    }

    pub fn sound_edit_begin(&mut self) {
        let current = match self.settings_cursor {
            8 => self
                .settings_edit
                .sound_work_to_break
                .as_deref()
                .unwrap_or(""),
            9 => self
                .settings_edit
                .sound_break_to_work
                .as_deref()
                .unwrap_or(""),
            _ => return,
        };
        self.sound_edit_buf = current.to_string();
        self.sound_editing = Some(match self.settings_cursor {
            8 => SoundField::WorkToBreak,
            _ => SoundField::BreakToWork,
        });
    }

    pub fn sound_edit_push(&mut self, c: char) {
        self.sound_edit_buf.push(c);
    }

    pub fn sound_edit_pop(&mut self) {
        self.sound_edit_buf.pop();
    }

    pub fn sound_edit_confirm(&mut self) {
        let path = self.sound_edit_buf.trim().to_string();
        let value = if path.is_empty() { None } else { Some(path) };
        match self.sound_editing {
            Some(SoundField::WorkToBreak) => self.settings_edit.sound_work_to_break = value,
            Some(SoundField::BreakToWork) => self.settings_edit.sound_break_to_work = value,
            None => {}
        }
        self.sound_editing = None;
        self.sound_edit_buf = String::new();
    }

    pub fn sound_edit_cancel(&mut self) {
        self.sound_editing = None;
        self.sound_edit_buf = String::new();
    }

    pub fn update(&mut self) {
        if matches!(self.timer_state, TimerState::Running | TimerState::Break) {
            if let Some(start) = self.session_start {
                self.elapsed = start.elapsed();

                if self.timer_state == TimerState::Running && self.elapsed > self.work_duration {
                    let proj = self
                        .selected_project()
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    let task = self
                        .selected_task()
                        .map(|t| t.name.clone())
                        .unwrap_or_default();
                    self.work_sessions_completed += 1;
                    let is_long_break = self
                        .work_sessions_completed
                        .is_multiple_of(self.sessions_before_long_break);

                    let svc = EntryService::new(self.storage, self.user_id);
                    if let Ok(entry) = svc.stop(None) {
                        let proj_name = if proj.is_empty() { None } else { Some(proj.as_str()) };
                        let task_name = if task.is_empty() { None } else { Some(task.as_str()) };
                        let _ = obsidian_logger::log_activity_to_obsidian(
                            &self.config,
                            &entry,
                            proj_name,
                            task_name,
                            obsidian_logger::ActivityAction::Stopped,
                        );
                    }

                    if self.auto_start_break {
                        self.timer_state = TimerState::Break;
                        self.session_start =
                            Some(Instant::now() - (self.elapsed - self.work_duration));
                    } else {
                        self.timer_state = TimerState::Paused;
                        self.paused_at = Some(Instant::now());
                    }

                    let break_msg = if is_long_break {
                        "Work session complete! Time for a long break."
                    } else {
                        "Work session complete! Short break time."
                    };
                    let sound = self.sound_work_to_break.clone();
                    self.notify("Pomodoro", break_msg, sound.as_deref());
                }

                let is_long_break = self.work_sessions_completed > 0
                    && self
                        .work_sessions_completed
                        .is_multiple_of(self.sessions_before_long_break);
                let current_break_duration = if is_long_break {
                    self.long_break_duration
                } else {
                    self.break_duration
                };
                let total_duration = self.work_duration + current_break_duration;
                if self.elapsed > total_duration {
                    let is_after_long_break = is_long_break;
                    let sound = self.sound_break_to_work.clone();
                    self.reset();
                    let msg = if is_after_long_break {
                        let cycles_done =
                            self.work_sessions_completed / self.sessions_before_long_break;
                        if self.max_cycles > 0 && cycles_done >= self.max_cycles {
                            format!("All {} cycles complete! Great work!", self.max_cycles)
                        } else {
                            "Long break complete! Ready for the next cycle.".to_string()
                        }
                    } else {
                        "Break complete! Ready for the next work session.".to_string()
                    };
                    self.notify("Pomodoro", &msg, sound.as_deref());
                }
            }
        }

        if let Some(set_at) = self.message_set_at {
            if !self.message_timeout.is_zero() && set_at.elapsed() > self.message_timeout {
                self.message = None;
                self.message_set_at = None;
            }
        }
    }

    pub fn can_quit(&self) -> bool {
        self.timer_state == TimerState::Stopped
    }

    fn refresh_tasks(&mut self) {
        if let Some(proj) = self.selected_project() {
            self.tasks = self.storage.list_tasks(&proj.id, false).unwrap_or_default();
            self.selected_task_idx = 0;
        } else {
            self.tasks.clear();
        }
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.projects.get(self.selected_project_idx)
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected_task_idx)
    }

    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    pub fn selected_project_idx(&self) -> usize {
        self.selected_project_idx
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn selected_task_idx(&self) -> usize {
        self.selected_task_idx
    }

    pub fn timer_state(&self) -> TimerState {
        self.timer_state
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn work_duration(&self) -> u64 {
        if self.timer_state == TimerState::Break {
            let is_long_break = self.work_sessions_completed > 0
                && self
                    .work_sessions_completed
                    .is_multiple_of(self.sessions_before_long_break);
            if is_long_break {
                self.long_break_duration.as_secs()
            } else {
                self.break_duration.as_secs()
            }
        } else {
            self.work_duration.as_secs()
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn sessions_completed(&self) -> u64 {
        self.work_sessions_completed
    }

    pub fn sessions_before_long(&self) -> u64 {
        self.sessions_before_long_break
    }

    pub fn is_new_task_editing(&self) -> bool {
        self.new_task_editing
    }

    pub fn new_task_buf(&self) -> &str {
        &self.new_task_buf
    }

    pub fn new_task_begin(&mut self) {
        if self.timer_state == TimerState::Stopped && self.selected_project().is_some() {
            self.new_task_editing = true;
            self.new_task_buf = String::new();
        }
    }

    pub fn new_task_push(&mut self, c: char) {
        self.new_task_buf.push(c);
    }

    pub fn new_task_pop(&mut self) {
        self.new_task_buf.pop();
    }

    pub fn new_task_confirm(&mut self) -> Result<()> {
        let name = self.new_task_buf.trim().to_string();
        if name.is_empty() {
            self.new_task_editing = false;
            self.new_task_buf = String::new();
            return Ok(());
        }
        if let Some(proj) = self.selected_project() {
            let new_task = NewTask {
                user_id: self.user_id.to_string(),
                project_id: proj.id.clone(),
                name: name.clone(),
                description: None,
            };
            self.storage.create_task(new_task)?;
            self.refresh_tasks();
            // select the newly created task (it will be last after refresh)
            if !self.tasks.is_empty() {
                if let Some(idx) = self.tasks.iter().position(|t| t.name == name) {
                    self.selected_task_idx = idx;
                }
            }
        }
        self.new_task_editing = false;
        self.new_task_buf = String::new();
        Ok(())
    }

    pub fn new_task_cancel(&mut self) {
        self.new_task_editing = false;
        self.new_task_buf = String::new();
    }

    pub fn task_complete_toggle(&mut self) -> Result<()> {
        if self.timer_state != TimerState::Stopped {
            return Ok(());
        }
        if let Some(task) = self.tasks.get(self.selected_task_idx) {
            let new_state = !task.completed;
            use tmkpr_lib::models::task::UpdateTask;
            self.storage.update_task(
                &task.id.clone(),
                UpdateTask {
                    completed: Some(new_state),
                    ..Default::default()
                },
            )?;
            self.refresh_tasks();
            self.message = Some(if new_state {
                "Task marked completed.".to_string()
            } else {
                "Task reactivated.".to_string()
            });
        }
        Ok(())
    }

    pub fn new_project_begin(&mut self) {
        if self.timer_state == TimerState::Stopped {
            self.new_project_editing = true;
            self.new_project_buf = String::new();
        }
    }

    pub fn new_project_push(&mut self, c: char) {
        self.new_project_buf.push(c);
    }

    pub fn new_project_pop(&mut self) {
        self.new_project_buf.pop();
    }

    pub fn new_project_confirm(&mut self) -> Result<()> {
        let name = self.new_project_buf.trim().to_string();
        if name.is_empty() {
            self.new_project_editing = false;
            self.new_project_buf = String::new();
            return Ok(());
        }
        use tmkpr_lib::models::project::NewProject;
        let new_proj = NewProject {
            user_id: self.user_id.to_string(),
            name: name.clone(),
            description: None,
            color: None,
        };
        self.storage.create_project(new_proj)?;
        self.projects = self.storage.list_projects(self.user_id, false).unwrap_or_default();
        if let Some(idx) = self.projects.iter().position(|p| p.name == name) {
            self.selected_project_idx = idx;
            self.refresh_tasks();
        }
        self.new_project_editing = false;
        self.new_project_buf = String::new();
        self.message = Some("Project created.".to_string());
        Ok(())
    }

    pub fn new_project_cancel(&mut self) {
        self.new_project_editing = false;
        self.new_project_buf = String::new();
    }

    pub fn delete_project_begin(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.projects.is_empty() {
            self.edit_mode = EditMode::ConfirmDelete;
            self.delete_target = Some("project");
        }
    }

    pub fn delete_task_begin(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.tasks.is_empty() {
            self.edit_mode = EditMode::ConfirmDelete;
            self.delete_target = Some("task");
        }
    }

    pub fn confirm_delete(&mut self) -> Result<()> {
        match self.delete_target {
            Some("project") => {
                if let Some(proj) = self.selected_project() {
                    self.storage.delete_project(&proj.id)?;
                    self.projects = self.storage.list_projects(self.user_id, false).unwrap_or_default();
                    self.selected_project_idx = self.selected_project_idx.min(self.projects.len().saturating_sub(1));
                    self.refresh_tasks();
                    self.message = Some("Project deleted.".to_string());
                }
            }
            Some("task") => {
                if let Some(task) = self.tasks.get(self.selected_task_idx) {
                    self.storage.delete_task(&task.id)?;
                    self.refresh_tasks();
                    self.message = Some("Task deleted.".to_string());
                }
            }
            _ => {}
        }
        self.edit_mode = EditMode::None;
        self.delete_target = None;
        Ok(())
    }

    pub fn cancel_delete(&mut self) {
        self.edit_mode = EditMode::None;
        self.delete_target = None;
    }

    pub fn is_new_project_editing(&self) -> bool {
        self.new_project_editing
    }

    pub fn new_project_buf(&self) -> &str {
        &self.new_project_buf
    }

    pub fn edit_mode(&self) -> EditMode {
        self.edit_mode
    }

    pub fn delete_target(&self) -> Option<&str> {
        self.delete_target
    }

    pub fn edit_project_begin(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.projects.is_empty() {
            let name = self.projects[self.selected_project_idx].name.clone();
            self.edit_mode = EditMode::EditProject;
            self.edit_buf = name;
        }
    }

    pub fn edit_task_begin(&mut self) {
        if self.timer_state == TimerState::Stopped && !self.tasks.is_empty() {
            let name = self.tasks[self.selected_task_idx].name.clone();
            self.edit_mode = EditMode::EditTask;
            self.edit_buf = name;
        }
    }

    pub fn edit_push(&mut self, c: char) {
        self.edit_buf.push(c);
    }

    pub fn edit_pop(&mut self) {
        self.edit_buf.pop();
    }

    pub fn edit_confirm(&mut self) -> Result<()> {
        let new_name = self.edit_buf.trim().to_string();
        if new_name.is_empty() {
            self.edit_mode = EditMode::None;
            self.edit_buf.clear();
            return Ok(());
        }

        match self.edit_mode {
            EditMode::EditProject => {
                if let Some(proj) = self.selected_project() {
                    use tmkpr_lib::models::project::UpdateProject;
                    self.storage.update_project(
                        &proj.id.clone(),
                        UpdateProject {
                            name: Some(new_name.clone()),
                            ..Default::default()
                        },
                    )?;
                    self.projects = self.storage.list_projects(self.user_id, false).unwrap_or_default();
                    self.message = Some("Project updated.".to_string());
                }
            }
            EditMode::EditTask => {
                if let Some(task) = self.selected_task() {
                    use tmkpr_lib::models::task::UpdateTask;
                    self.storage.update_task(
                        &task.id.clone(),
                        UpdateTask {
                            name: Some(new_name.clone()),
                            ..Default::default()
                        },
                    )?;
                    self.refresh_tasks();
                    self.message = Some("Task updated.".to_string());
                }
            }
            _ => {}
        }

        self.edit_mode = EditMode::None;
        self.edit_buf.clear();
        Ok(())
    }

    pub fn edit_cancel(&mut self) {
        self.edit_mode = EditMode::None;
        self.edit_buf.clear();
    }

    pub fn edit_buf(&self) -> &str {
        &self.edit_buf
    }

    pub fn toggle_hide_completed_tasks(&mut self) -> Result<()> {
        self.hide_completed_tasks = !self.hide_completed_tasks;
        self.config.pomodoro.hide_completed_tasks = self.hide_completed_tasks;
        self.config.save()?;

        let status = if self.hide_completed_tasks {
            "Hiding completed tasks."
        } else {
            "Showing completed tasks."
        };
        self.message = Some(status.to_string());
        Ok(())
    }

    pub fn hide_completed_tasks(&self) -> bool {
        self.hide_completed_tasks
    }

    pub fn filtered_tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| !self.hide_completed_tasks || !t.completed)
            .collect()
    }

    pub fn add_selected_task_to_queue(&mut self) {
        let task_to_add = self.selected_task().cloned();
        if let Some(task) = task_to_add {
            if !self.task_queue.iter().any(|t| t.id == task.id) {
                self.task_queue.push(task.clone());
                self.message = Some(format!("Added '{}' to queue.", task.name));
            } else {
                self.message = Some("Task already in queue.".to_string());
            }
        }
    }

    pub fn remove_from_queue(&mut self) {
        if !self.task_queue.is_empty() {
            let removed = self.task_queue.remove(self.selected_queue_idx);
            self.selected_queue_idx = self.selected_queue_idx.saturating_sub(1);
            self.message = Some(format!("Removed '{}' from queue.", removed.name));
        }
    }

    pub fn move_queue_up(&mut self) {
        if !self.task_queue.is_empty() && self.selected_queue_idx > 0 {
            self.task_queue.swap(self.selected_queue_idx, self.selected_queue_idx - 1);
            self.selected_queue_idx -= 1;
        }
    }

    pub fn move_queue_down(&mut self) {
        if !self.task_queue.is_empty() && self.selected_queue_idx < self.task_queue.len() - 1 {
            self.task_queue.swap(self.selected_queue_idx, self.selected_queue_idx + 1);
            self.selected_queue_idx += 1;
        }
    }

    pub fn select_next_queue(&mut self) {
        if !self.task_queue.is_empty() {
            self.selected_queue_idx = (self.selected_queue_idx + 1) % self.task_queue.len();
        }
    }

    pub fn select_prev_queue(&mut self) {
        if !self.task_queue.is_empty() {
            self.selected_queue_idx = if self.selected_queue_idx == 0 {
                self.task_queue.len() - 1
            } else {
                self.selected_queue_idx - 1
            };
        }
    }

    pub fn task_queue(&self) -> &[Task] {
        &self.task_queue
    }

    pub fn selected_queue_idx(&self) -> usize {
        self.selected_queue_idx
    }

    pub fn start_queue_task(&mut self) -> Result<()> {
        if self.task_queue.is_empty() {
            self.message = Some("Queue is empty.".to_string());
            return Ok(());
        }

        let task = self.task_queue[self.selected_queue_idx].clone();
        let project = self.projects.iter().find(|p| {
            self.storage
                .list_tasks(&p.id, false)
                .unwrap_or_default()
                .iter()
                .any(|t| t.id == task.id)
        });

        if let Some(proj) = project {
            if self.timer_state == TimerState::Stopped {
                self.timer_state = TimerState::Running;
                self.session_start = Some(Instant::now());
                self.elapsed = Duration::ZERO;
                let msg = format!("Timer started! Working on '{}'", task.name);
                self.message = Some(msg);

                let svc = EntryService::new(self.storage, self.user_id);
                if let Ok(_entry) = svc.start(Some(&proj.name), Some(&task.name), None, vec![], None) {
                    let _ = obsidian_logger::log_activity_to_obsidian(
                        &self.config,
                        &_entry,
                        Some(&proj.name),
                        Some(&task.name),
                        obsidian_logger::ActivityAction::Started,
                    );
                }

                self.remove_from_queue();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmkpr_lib::{config::Config, storage::sqlite::SqliteStorage};

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn make_app(s: &dyn Storage) -> App<'_> {
        App::new(
            s,
            "local",
            Config::default(),
            crate::theme::Theme::from_name("default"),
        )
        .unwrap()
    }

    // --- timer state ---

    #[test]
    fn initial_state_stopped_and_can_quit() {
        let s = storage();
        let app = make_app(&s);
        assert_eq!(app.timer_state(), TimerState::Stopped);
        assert!(app.can_quit());
    }

    #[test]
    fn start_timer_transitions_to_running() {
        let s = storage();
        let mut app = make_app(&s);
        app.start_timer().unwrap();
        assert_eq!(app.timer_state(), TimerState::Running);
        assert!(!app.can_quit());
    }

    #[test]
    fn toggle_cycles_running_paused_running() {
        let s = storage();
        let mut app = make_app(&s);
        app.start_timer().unwrap();
        app.toggle_timer();
        assert_eq!(app.timer_state(), TimerState::Paused);
        app.toggle_timer();
        assert_eq!(app.timer_state(), TimerState::Running);
    }

    #[test]
    fn toggle_noop_when_stopped() {
        let s = storage();
        let mut app = make_app(&s);
        app.toggle_timer();
        assert_eq!(app.timer_state(), TimerState::Stopped);
    }

    #[test]
    fn reset_returns_to_stopped_with_zero_elapsed() {
        let s = storage();
        let mut app = make_app(&s);
        app.start_timer().unwrap();
        app.reset();
        assert_eq!(app.timer_state(), TimerState::Stopped);
        assert_eq!(app.elapsed(), Duration::ZERO);
    }

    // --- settings cursor ---

    #[test]
    fn cursor_down_wraps_from_9_to_0() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..9 {
            app.settings_cursor_down();
        }
        assert_eq!(app.settings_state().1, 9);
        app.settings_cursor_down();
        assert_eq!(app.settings_state().1, 0);
    }

    #[test]
    fn cursor_up_wraps_from_0_to_9() {
        let s = storage();
        let mut app = make_app(&s);
        assert_eq!(app.settings_state().1, 0);
        app.settings_cursor_up();
        assert_eq!(app.settings_state().1, 9);
    }

    #[test]
    fn sound_field_boundary_at_cursor_8() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..7 {
            app.settings_cursor_down();
        }
        assert_eq!(app.settings_state().1, 7);
        assert!(!app.settings_cursor_on_sound_field());
        app.settings_cursor_down();
        assert_eq!(app.settings_state().1, 8);
        assert!(app.settings_cursor_on_sound_field());
        app.settings_cursor_down();
        assert!(app.settings_cursor_on_sound_field());
    }

    // --- settings adjust ---

    #[test]
    fn work_duration_clamps_at_1() {
        let s = storage();
        let mut app = make_app(&s);
        // cursor 0 = work_duration
        app.settings_adjust(-9999);
        assert_eq!(app.settings_state().0.work_duration_minutes, 1);
    }

    #[test]
    fn max_cycles_clamps_at_0() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..4 {
            app.settings_cursor_down();
        } // cursor 4 = max_cycles
        app.settings_adjust(-9999);
        assert_eq!(app.settings_state().0.max_cycles, 0);
    }

    #[test]
    fn max_cycles_increments_and_decrements() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..4 {
            app.settings_cursor_down();
        }
        app.settings_adjust(3);
        assert_eq!(app.settings_state().0.max_cycles, 3);
        app.settings_adjust(-1);
        assert_eq!(app.settings_state().0.max_cycles, 2);
    }

    #[test]
    fn notify_desktop_toggles_on_any_nonzero_delta() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..5 {
            app.settings_cursor_down();
        } // cursor 5 = notify_desktop
        let initial = app.settings_state().0.notify_desktop;
        app.settings_adjust(1);
        assert_eq!(app.settings_state().0.notify_desktop, !initial);
        app.settings_adjust(-1);
        assert_eq!(app.settings_state().0.notify_desktop, initial);
    }

    #[test]
    fn adjust_noop_on_sound_field_cursors() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        } // cursor 8 = sound field
        let before = app.settings_state().0.sound_work_to_break.clone();
        app.settings_adjust(1);
        assert_eq!(app.settings_state().0.sound_work_to_break, before);
    }

    // --- sound edit ---

    #[test]
    fn sound_edit_begin_populates_buf_from_current_value() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        }
        // first set a value via confirm
        app.sound_edit_begin();
        app.sound_edit_push('/');
        app.sound_edit_push('a');
        app.sound_edit_confirm();
        // now begin again — buf should be pre-populated
        app.sound_edit_begin();
        assert_eq!(app.sound_edit_buf(), "/a");
    }

    #[test]
    fn sound_edit_confirm_updates_work_to_break_path() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        }
        app.sound_edit_begin();
        for c in "/sounds/ding.wav".chars() {
            app.sound_edit_push(c);
        }
        app.sound_edit_confirm();
        assert_eq!(
            app.settings_state().0.sound_work_to_break.as_deref(),
            Some("/sounds/ding.wav"),
        );
    }

    #[test]
    fn sound_edit_empty_confirm_sets_none() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        }
        app.sound_edit_begin();
        app.sound_edit_confirm(); // confirm with empty buf
        assert_eq!(app.settings_state().0.sound_work_to_break, None);
    }

    #[test]
    fn sound_edit_cancel_discards_typed_chars() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        }
        app.sound_edit_begin();
        app.sound_edit_push('x');
        app.sound_edit_cancel();
        assert!(!app.is_editing_sound());
        assert_eq!(app.settings_state().0.sound_work_to_break, None);
    }

    #[test]
    fn sound_edit_pop_removes_last_char() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..8 {
            app.settings_cursor_down();
        }
        app.sound_edit_begin();
        app.sound_edit_push('a');
        app.sound_edit_push('b');
        app.sound_edit_pop();
        assert_eq!(app.sound_edit_buf(), "a");
    }

    #[test]
    fn break_to_work_uses_break_to_work_sound_cursor() {
        let s = storage();
        let mut app = make_app(&s);
        for _ in 0..9 {
            app.settings_cursor_down();
        } // cursor 9 = sound break→work
        app.sound_edit_begin();
        for c in "/sounds/chime.wav".chars() {
            app.sound_edit_push(c);
        }
        app.sound_edit_confirm();
        assert_eq!(
            app.settings_state().0.sound_break_to_work.as_deref(),
            Some("/sounds/chime.wav"),
        );
        assert_eq!(app.settings_state().0.sound_work_to_break, None);
    }

    // ── task completion ───────────────────────────────────────────────────────

    fn seed_project_and_task(s: &dyn Storage) {
        use tmkpr_lib::models::{project::NewProject, task::NewTask, LOCAL_USER_ID};
        let p = s
            .create_project(NewProject {
                user_id: LOCAL_USER_ID.to_string(),
                name: "proj".to_string(),
                description: None,
                color: None,
            })
            .unwrap();
        s.create_task(NewTask {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: p.id.clone(),
            name: "task".to_string(),
            description: None,
        })
        .unwrap();
    }

    fn seed_and_make_app(s: &dyn Storage) -> App<'_> {
        seed_project_and_task(s);
        App::new(
            s,
            tmkpr_lib::models::LOCAL_USER_ID,
            Config::default(),
            crate::theme::Theme::from_name("default"),
        )
        .unwrap()
    }

    #[test]
    fn task_complete_toggle_marks_task_done() {
        let s = storage();
        let mut app = seed_and_make_app(&s);
        assert_eq!(app.tasks().len(), 1);
        assert!(!app.tasks()[0].completed);

        app.task_complete_toggle().unwrap();

        assert!(app.tasks()[0].completed);
        assert_eq!(app.message(), Some("Task marked completed."));
    }

    #[test]
    fn task_complete_toggle_reactivates_completed_task() {
        let s = storage();
        let mut app = seed_and_make_app(&s);
        app.task_complete_toggle().unwrap();
        app.task_complete_toggle().unwrap();
        assert!(!app.tasks()[0].completed);
        assert_eq!(app.message(), Some("Task reactivated."));
    }

    #[test]
    fn start_timer_blocked_when_task_completed() {
        let s = storage();
        let mut app = seed_and_make_app(&s);
        app.task_complete_toggle().unwrap();
        assert!(app.tasks()[0].completed);

        app.start_timer().unwrap();

        assert_eq!(app.timer_state(), TimerState::Stopped);
        assert_eq!(
            app.message(),
            Some("Task is completed. Reactivate it first.")
        );
    }

    #[test]
    fn start_timer_allowed_after_reactivation() {
        let s = storage();
        let mut app = seed_and_make_app(&s);
        app.task_complete_toggle().unwrap();
        app.task_complete_toggle().unwrap();

        app.start_timer().unwrap();
        assert_eq!(app.timer_state(), TimerState::Running);
    }

    #[test]
    fn task_complete_toggle_noop_when_timer_running() {
        let s = storage();
        let mut app = seed_and_make_app(&s);
        app.start_timer().unwrap();
        assert_eq!(app.timer_state(), TimerState::Running);

        app.task_complete_toggle().unwrap();

        assert!(!app.tasks()[0].completed);
    }
}
