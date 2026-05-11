use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub num_id: u32,
}

#[derive(Debug, Clone)]
pub struct NewTask {
    pub user_id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTask {
    pub name: Option<String>,
    /// `Some(None)` clears the field; `None` leaves it unchanged.
    pub description: Option<Option<String>>,
    pub archived: Option<bool>,
    /// Move task to a different project (UUID).
    pub project_id: Option<String>,
}
