use std::path::Path;

use chrono::{DateTime, Utc};

use crate::error::TmkprResult;
use crate::models::{
    comment::{Comment, NewComment},
    entry::{Entry, EntryFilter, NewEntry, UpdateEntry},
    project::{NewProject, Project, UpdateProject},
    task::{NewTask, Task, UpdateTask},
};

pub trait Storage: Send + Sync {
    // ── Projects ─────────────────────────────────────────────────────────────
    fn create_project(&self, project: NewProject) -> TmkprResult<Project>;
    fn get_project(&self, id: &str) -> TmkprResult<Project>;
    fn get_project_by_name(&self, user_id: &str, name: &str) -> TmkprResult<Option<Project>>;
    fn get_project_by_num_id(&self, user_id: &str, num_id: u32) -> TmkprResult<Option<Project>>;
    fn list_projects(&self, user_id: &str, include_archived: bool) -> TmkprResult<Vec<Project>>;
    fn update_project(&self, id: &str, update: UpdateProject) -> TmkprResult<Project>;
    fn delete_project(&self, id: &str) -> TmkprResult<()>;

    // ── Tasks ────────────────────────────────────────────────────────────────
    fn create_task(&self, task: NewTask) -> TmkprResult<Task>;
    fn get_task(&self, id: &str) -> TmkprResult<Task>;
    fn get_task_by_name(&self, project_id: &str, name: &str) -> TmkprResult<Option<Task>>;
    fn get_task_by_num_id(&self, project_id: &str, num_id: u32) -> TmkprResult<Option<Task>>;
    fn list_tasks(&self, project_id: &str, include_archived: bool) -> TmkprResult<Vec<Task>>;
    fn list_all_tasks(&self, user_id: &str, include_archived: bool) -> TmkprResult<Vec<Task>>;
    fn update_task(&self, id: &str, update: UpdateTask) -> TmkprResult<Task>;
    fn delete_task(&self, id: &str) -> TmkprResult<()>;

    // ── Entries ──────────────────────────────────────────────────────────────
    fn create_entry(&self, entry: NewEntry) -> TmkprResult<Entry>;
    fn get_entry(&self, id: &str) -> TmkprResult<Entry>;
    /// Returns the entry where `finished_at IS NULL`, if any.
    fn get_active_entry(&self, user_id: &str) -> TmkprResult<Option<Entry>>;
    fn list_entries(&self, filter: &EntryFilter) -> TmkprResult<Vec<Entry>>;
    fn update_entry(&self, id: &str, update: UpdateEntry) -> TmkprResult<Entry>;
    fn delete_entry(&self, id: &str) -> TmkprResult<()>;
    /// Set `finished_at` on the currently active entry.
    fn finish_entry(&self, user_id: &str, finished_at: DateTime<Utc>) -> TmkprResult<Entry>;
    /// Resolve a UUID prefix to a full entry ID. Errors if 0 or >1 match.
    fn resolve_entry_id(&self, user_id: &str, prefix: &str) -> TmkprResult<String>;

    // ── Tags ─────────────────────────────────────────────────────────────────
    /// Returns all tags in use across finished entries, sorted by count desc then name asc.
    fn list_tags(&self, user_id: &str) -> TmkprResult<Vec<(String, usize)>>;

    // ── Comments ─────────────────────────────────────────────────────────────
    fn create_comment(&self, comment: NewComment) -> TmkprResult<Comment>;
    fn get_comment(&self, id: &str) -> TmkprResult<Comment>;
    fn list_comments(&self, entry_id: &str) -> TmkprResult<Vec<Comment>>;
    fn update_comment(&self, id: &str, body: String) -> TmkprResult<Comment>;
    fn delete_comment(&self, id: &str) -> TmkprResult<()>;
    /// Resolve a UUID prefix to a full comment ID scoped to the user's entries.
    fn resolve_comment_id(&self, user_id: &str, prefix: &str) -> TmkprResult<String>;
}

pub fn open_sqlite(db_path: &Path) -> TmkprResult<Box<dyn Storage>> {
    use sqlite::SqliteStorage;
    let storage = SqliteStorage::open(db_path)?;
    Ok(Box::new(storage))
}

pub mod sqlite;
