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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_entry(started_at: DateTime<Utc>, finished_at: Option<DateTime<Utc>>) -> Entry {
        Entry {
            id: "test-id".to_string(),
            user_id: "u1".to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at,
            finished_at,
            tags: vec![],
            created_at: started_at,
            updated_at: started_at,
        }
    }

    #[test]
    fn parse_tags_splits_and_trims() {
        assert_eq!(parse_tags("foo, bar, baz"), vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn parse_tags_filters_empty() {
        assert_eq!(parse_tags("foo,,bar"), vec!["foo", "bar"]);
        assert_eq!(parse_tags(""), Vec::<String>::new());
        assert_eq!(parse_tags("  ,  "), Vec::<String>::new());
    }

    #[test]
    fn parse_tags_single() {
        assert_eq!(parse_tags("only"), vec!["only"]);
    }

    #[test]
    fn is_active_true_when_no_finished_at() {
        let e = make_entry(Utc::now(), None);
        assert!(e.is_active());
    }

    #[test]
    fn is_active_false_when_finished() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let e = make_entry(start, Some(end));
        assert!(!e.is_active());
    }

    #[test]
    fn duration_none_for_active() {
        let e = make_entry(Utc::now(), None);
        assert!(e.duration().is_none());
    }

    #[test]
    fn duration_some_for_finished() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 1, 1, 10, 30, 0).unwrap();
        let e = make_entry(start, Some(end));
        assert_eq!(e.duration().unwrap(), Duration::minutes(90));
    }

    #[test]
    fn elapsed_equals_duration_for_finished() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 8, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        let e = make_entry(start, Some(end));
        assert_eq!(e.elapsed(), Duration::hours(1));
    }
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
