use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

pub const NO_PROJECT: &str = "(no project)";
pub const NO_TASK: &str = "(no task)";

pub fn parse_tags(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub user_id: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub note: Option<String>,
    pub started_at: DateTime<Utc>,
    /// `None` means this entry is currently active.
    pub finished_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entry {
    pub fn is_active(&self) -> bool {
        self.finished_at.is_none()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.finished_at
            .map(|f| f.signed_duration_since(self.started_at))
    }

    pub fn elapsed(&self) -> Duration {
        let end = self.finished_at.unwrap_or_else(Utc::now);
        end.signed_duration_since(self.started_at)
    }
}

#[derive(Debug, Clone)]
pub struct NewEntry {
    pub user_id: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub note: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateEntry {
    /// `Some(None)` clears the field; `None` leaves it unchanged.
    pub project_id: Option<Option<String>>,
    pub task_id: Option<Option<String>>,
    pub note: Option<Option<String>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<Option<DateTime<Utc>>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub user_id: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    /// All listed tags must be present on the entry (AND semantics).
    pub tags: Vec<String>,
    pub limit: Option<u32>,
    /// When false, active entries (finished_at IS NULL) are excluded.
    pub include_active: bool,
}
