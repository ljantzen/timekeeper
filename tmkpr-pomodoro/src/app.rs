use anyhow::Result;
use std::time::{Duration, Instant};
use tmkpr_lib::{
    config::{Config, PomodoroConfig}, models::project::Project, models::task::Task, service::EntryService,
    storage::Storage,
};

#[derive(Clone, Debug)]
pub struct CompletedSession {
    pub project: String,
    pub task: String,
    pub duration: Duration,
    pub color: Option<String>,
}

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
    screen: Screen,
    config: Config,
    settings_edit: PomodoroConfig,
    settings_cursor: usize,
    sound_editing: Option<SoundField>,
    sound_edit_buf: String,
    completed_sessions: Vec<CompletedSession>,
}

impl<'a> App<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str, config: Config) -> Result<Self> {
        let projects = storage.list_projects(user_id, false).unwrap_or_default();
        let tasks = if !projects.is_empty() {
            storage
                .list_tasks(&projects[0].id, false)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

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
            long_break_duration: Duration::from_secs(config.pomodoro.long_break_duration_minutes * 60),
            sessions_before_long_break: config.pomodoro.sessions_before_long_break,
            work_sessions_completed: 0,
            notify_desktop: config.pomodoro.notify_desktop,
            sound_work_to_break: config.pomodoro.sound_work_to_break.clone(),
            sound_break_to_work: config.pomodoro.sound_break_to_work.clone(),
            message_timeout: Duration::from_secs(config.pomodoro.message_timeout_secs),
            message_set_at: None,
            auto_start_break: config.pomodoro.auto_start_break,
            screen: Screen::Main,
            settings_edit: config.pomodoro.clone(),
            config,
            settings_cursor: 0,
            sound_editing: None,
            sound_edit_buf: String::new(),
            completed_sessions: Vec::new(),
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
            self.timer_state = TimerState::Running;
            self.session_start = Some(Instant::now());
            self.elapsed = Duration::ZERO;
            self.message = Some("Timer started! Press space to pause.".to_string());

            let svc = EntryService::new(self.storage, self.user_id);
            if let Some(proj) = self.selected_project() {
                if let Some(task) = self.selected_task() {
                    let _ = svc.start(Some(&proj.name), Some(&task.name), None, vec![], None);
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
            let is_long_break = self.work_sessions_completed.is_multiple_of(self.sessions_before_long_break);

            if is_long_break {
                let svc = EntryService::new(self.storage, self.user_id);
                let _ = svc.stop(None);
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
            let elapsed = self.elapsed;
            let proj = self.selected_project().map(|p| p.name.clone()).unwrap_or_default();
            let task = self.selected_task().map(|t| t.name.clone()).unwrap_or_default();
            let color = self.selected_project().and_then(|p| p.color.clone());

            let svc = EntryService::new(self.storage, self.user_id);
            svc.stop(None)?;

            self.completed_sessions.push(CompletedSession { project: proj, task, duration: elapsed, color });
            if self.completed_sessions.len() > 20 {
                self.completed_sessions.remove(0);
            }

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
        let _ = svc.stop(None);

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
        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        let Ok(file) = std::fs::File::open(&path) else { return };
        let Ok(source) = rodio::Decoder::new(std::io::BufReader::new(file)) else { return };
        sink.append(source);
        sink.sleep_until_end();
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
            self.settings_cursor = 8;
        } else {
            self.settings_cursor -= 1;
        }
    }

    pub fn settings_cursor_down(&mut self) {
        if self.settings_cursor == 8 {
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
            4 if delta != 0 => {
                self.settings_edit.notify_desktop = !self.settings_edit.notify_desktop;
            }
            5 => {
                self.settings_edit.message_timeout_secs =
                    (self.settings_edit.message_timeout_secs as i64 + delta).max(0) as u64;
            }
            6 if delta != 0 => {
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
        self.long_break_duration = Duration::from_secs(self.config.pomodoro.long_break_duration_minutes * 60);
        self.sessions_before_long_break = self.config.pomodoro.sessions_before_long_break;
        self.notify_desktop = self.config.pomodoro.notify_desktop;
        self.message_timeout = Duration::from_secs(self.config.pomodoro.message_timeout_secs);
        self.auto_start_break = self.config.pomodoro.auto_start_break;
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
        self.settings_cursor >= 7
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
            7 => self.settings_edit.sound_work_to_break.as_deref().unwrap_or(""),
            8 => self.settings_edit.sound_break_to_work.as_deref().unwrap_or(""),
            _ => return,
        };
        self.sound_edit_buf = current.to_string();
        self.sound_editing = Some(match self.settings_cursor {
            7 => SoundField::WorkToBreak,
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
                    let proj = self.selected_project().map(|p| p.name.clone()).unwrap_or_default();
                    let task = self.selected_task().map(|t| t.name.clone()).unwrap_or_default();
                    let color = self.selected_project().and_then(|p| p.color.clone());
                    self.completed_sessions.push(CompletedSession {
                        project: proj,
                        task,
                        duration: self.work_duration,
                        color,
                    });
                    if self.completed_sessions.len() > 20 {
                        self.completed_sessions.remove(0);
                    }

                    self.work_sessions_completed += 1;
                    let is_long_break = self.work_sessions_completed.is_multiple_of(self.sessions_before_long_break);

                    if is_long_break {
                        let svc = EntryService::new(self.storage, self.user_id);
                        let _ = svc.stop(None);
                    }

                    if self.auto_start_break {
                        self.timer_state = TimerState::Break;
                        self.session_start = Some(Instant::now() - (self.elapsed - self.work_duration));
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
                    && self.work_sessions_completed.is_multiple_of(self.sessions_before_long_break);
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
                        "Long break complete! Ready for the next cycle.".to_string()
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

    pub fn work_duration(&self) -> u64 {
        if self.timer_state == TimerState::Break {
            let is_long_break =
                self.work_sessions_completed > 0
                    && self.work_sessions_completed.is_multiple_of(self.sessions_before_long_break);
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

    pub fn completed_sessions(&self) -> &[CompletedSession] {
        &self.completed_sessions
    }
}
