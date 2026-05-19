use anyhow::Result;
use chrono::Utc;
use std::time::{Duration, Instant};
use tmkpr_lib::{
    config::Config, models::project::Project, models::task::Task, service::EntryService,
    storage::Storage,
};

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
}

impl<'a> App<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str, config: &Config) -> Result<Self> {
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
            let project = self.selected_project();
            let task = self.selected_task();

            if let (Some(proj), Some(t)) = (project, task) {
                let now = Utc::now();
                let started_at = now - chrono::Duration::seconds(self.elapsed.as_secs() as i64);
                let svc = EntryService::new(self.storage, self.user_id);
                svc.log(
                    Some(&proj.name),
                    Some(&t.name),
                    None,
                    vec![],
                    started_at,
                    now,
                )?;
                self.message = Some("Session logged!".to_string());
            } else {
                self.message = Some("Please select a project and task.".to_string());
            }

            self.reset();
            Ok(())
        } else {
            self.message = Some("No active session to log.".to_string());
            Ok(())
        }
    }

    pub fn reset(&mut self) {
        self.timer_state = TimerState::Stopped;
        self.elapsed = Duration::ZERO;
        self.session_start = None;
        self.paused_at = None;
    }

    pub fn update(&mut self) {
        if self.timer_state == TimerState::Running {
            if let Some(start) = self.session_start {
                self.elapsed = start.elapsed();

                if self.elapsed > self.work_duration {
                    self.work_sessions_completed += 1;
                    let is_long_break = self.work_sessions_completed.is_multiple_of(self.sessions_before_long_break);
                    self.timer_state = TimerState::Break;
                    self.session_start = Some(Instant::now() - (self.elapsed - self.work_duration));
                    let break_msg = if is_long_break {
                        "Work session complete! Time for a long break."
                    } else {
                        "Work session complete! Short break time."
                    };
                    self.message = Some(break_msg.to_string());
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
                    self.reset();
                    if is_after_long_break {
                        self.message = Some("Long break complete! Ready for the next cycle.".to_string());
                    } else {
                        self.message = Some("Break complete! Ready for the next work session.".to_string());
                    }
                }
            }
        }

        // Clear message after 3 seconds
        // (In a real app, track message_time)
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
}
