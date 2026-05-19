use anyhow::Result;
use chrono::Utc;
use std::time::{Duration, Instant};
use tmkpr_lib::{
    models::project::Project, models::task::Task, service::EntryService, storage::Storage,
};

const WORK_DURATION: u64 = 25 * 60; // 25 minutes
const BREAK_DURATION: u64 = 5 * 60; // 5 minutes

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
}

impl<'a> App<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str) -> Result<Self> {
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

                let total_work = Duration::from_secs(WORK_DURATION);
                if self.elapsed > total_work {
                    self.timer_state = TimerState::Break;
                    self.session_start = Some(Instant::now() - (self.elapsed - total_work));
                    self.message = Some("Work session complete! Break time.".to_string());
                }

                let total_duration = total_work + Duration::from_secs(BREAK_DURATION);
                if self.elapsed > total_duration {
                    self.reset();
                    self.message = Some("Session complete! Ready for the next one.".to_string());
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
            BREAK_DURATION
        } else {
            WORK_DURATION
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
