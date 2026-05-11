use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::error::{TmkprError, TmkprResult};
use crate::models::{
    entry::{Entry, EntryFilter, NewEntry, UpdateEntry},
    project::{NewProject, Project, UpdateProject},
    task::{NewTask, Task, UpdateTask},
    user::User,
};
use crate::storage::Storage;

pub mod migrations;

pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn open(path: &Path) -> TmkprResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;",
        )?;
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> TmkprResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

// ── Row mappers ──────────────────────────────────────────────────────────────

fn row_to_user(row: &Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn row_to_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    let archived: i64 = row.get(5)?;
    Ok(Project {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        color: row.get(4)?,
        archived: archived != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        num_id: row.get::<_, Option<u32>>(8)?.unwrap_or(0),
    })
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let archived: i64 = row.get(5)?;
    Ok(Task {
        id: row.get(0)?,
        user_id: row.get(1)?,
        project_id: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        archived: archived != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        num_id: row.get::<_, Option<u32>>(8)?.unwrap_or(0),
    })
}

fn row_to_entry(row: &Row<'_>) -> rusqlite::Result<Entry> {
    let tags_json: Option<String> = row.get(7)?;
    let tags: Vec<String> = tags_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(Entry {
        id: row.get(0)?,
        user_id: row.get(1)?,
        project_id: row.get(2)?,
        task_id: row.get(3)?,
        note: row.get(4)?,
        started_at: row.get(5)?,
        finished_at: row.get(6)?,
        tags,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn tags_to_json(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        serde_json::to_string(tags).ok()
    }
}

// ── Storage impl ─────────────────────────────────────────────────────────────

impl Storage for SqliteStorage {
    fn get_user(&self, user_id: &str) -> TmkprResult<User> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, display_name, created_at, updated_at
             FROM users WHERE id = ?1",
            params![user_id],
            row_to_user,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => TmkprError::NotFound {
                entity: "user",
                id: user_id.to_string(),
            },
            other => TmkprError::Database(other),
        })
    }

    // ── Projects ─────────────────────────────────────────────────────────────

    fn create_project(&self, p: NewProject) -> TmkprResult<Project> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, user_id, name, description, color, num_id)
             VALUES (?1, ?2, ?3, ?4, ?5,
                 (SELECT COALESCE(MAX(num_id), 0) + 1 FROM projects WHERE user_id = ?2))",
            params![id, p.user_id, p.name, p.description, p.color],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    return TmkprError::Conflict(format!("project `{}` already exists", p.name));
                }
            }
            TmkprError::Database(e)
        })?;
        drop(conn);
        self.get_project(&id)
    }

    fn get_project(&self, id: &str) -> TmkprResult<Project> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, description, color, archived, created_at, updated_at, num_id
             FROM projects WHERE id = ?1",
            params![id],
            row_to_project,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => TmkprError::NotFound {
                entity: "project",
                id: id.to_string(),
            },
            other => TmkprError::Database(other),
        })
    }

    fn get_project_by_name(&self, user_id: &str, name: &str) -> TmkprResult<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, description, color, archived, created_at, updated_at, num_id
             FROM projects WHERE user_id = ?1 AND name = ?2",
            params![user_id, name],
            row_to_project,
        )
        .optional()
        .map_err(TmkprError::Database)
    }

    fn get_project_by_num_id(&self, user_id: &str, num_id: u32) -> TmkprResult<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, description, color, archived, created_at, updated_at, num_id
             FROM projects WHERE user_id = ?1 AND num_id = ?2",
            params![user_id, num_id],
            row_to_project,
        )
        .optional()
        .map_err(TmkprError::Database)
    }

    fn list_projects(&self, user_id: &str, include_archived: bool) -> TmkprResult<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let archived_filter = if include_archived {
            "1=1"
        } else {
            "archived = 0"
        };
        let sql = format!(
            "SELECT id, user_id, name, description, color, archived, created_at, updated_at, num_id
             FROM projects WHERE user_id = ?1 AND {} ORDER BY num_id",
            archived_filter
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![user_id], row_to_project)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(TmkprError::Database)
    }

    fn update_project(&self, id: &str, u: UpdateProject) -> TmkprResult<Project> {
        let conn = self.conn.lock().unwrap();
        let mut sets: Vec<String> = vec!["updated_at = datetime('now')".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(name) = u.name {
            binds.push(Box::new(name));
            sets.push(format!("name = ?{}", binds.len()));
        }
        if let Some(desc) = u.description {
            binds.push(Box::new(desc));
            sets.push(format!("description = ?{}", binds.len()));
        }
        if let Some(color) = u.color {
            binds.push(Box::new(color));
            sets.push(format!("color = ?{}", binds.len()));
        }
        if let Some(archived) = u.archived {
            binds.push(Box::new(archived as i64));
            sets.push(format!("archived = ?{}", binds.len()));
        }

        binds.push(Box::new(id.to_string()));
        let id_pos = binds.len();
        let sql = format!(
            "UPDATE projects SET {} WHERE id = ?{}",
            sets.join(", "),
            id_pos
        );
        let params_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        drop(conn);
        self.get_project(id)
    }

    fn delete_project(&self, id: &str) -> TmkprResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Tasks ─────────────────────────────────────────────────────────────────

    fn create_task(&self, t: NewTask) -> TmkprResult<Task> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tasks (id, user_id, project_id, name, description, num_id)
             VALUES (?1, ?2, ?3, ?4, ?5,
                 (SELECT COALESCE(MAX(num_id), 0) + 1 FROM tasks WHERE project_id = ?3))",
            params![id, t.user_id, t.project_id, t.name, t.description],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(ref err, _) = e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    return TmkprError::Conflict(format!(
                        "task `{}` already exists in project",
                        t.name
                    ));
                }
            }
            TmkprError::Database(e)
        })?;
        drop(conn);
        self.get_task(&id)
    }

    fn get_task(&self, id: &str) -> TmkprResult<Task> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, name, description, archived, created_at, updated_at, num_id
             FROM tasks WHERE id = ?1",
            params![id],
            row_to_task,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => TmkprError::NotFound {
                entity: "task",
                id: id.to_string(),
            },
            other => TmkprError::Database(other),
        })
    }

    fn get_task_by_name(&self, project_id: &str, name: &str) -> TmkprResult<Option<Task>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, name, description, archived, created_at, updated_at, num_id
             FROM tasks WHERE project_id = ?1 AND name = ?2",
            params![project_id, name],
            row_to_task,
        )
        .optional()
        .map_err(TmkprError::Database)
    }

    fn get_task_by_num_id(&self, project_id: &str, num_id: u32) -> TmkprResult<Option<Task>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, name, description, archived, created_at, updated_at, num_id
             FROM tasks WHERE project_id = ?1 AND num_id = ?2",
            params![project_id, num_id],
            row_to_task,
        )
        .optional()
        .map_err(TmkprError::Database)
    }

    fn list_tasks(&self, project_id: &str, include_archived: bool) -> TmkprResult<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let archived_filter = if include_archived {
            "1=1"
        } else {
            "archived = 0"
        };
        let sql = format!(
            "SELECT id, user_id, project_id, name, description, archived, created_at, updated_at, num_id
             FROM tasks WHERE project_id = ?1 AND {} ORDER BY num_id",
            archived_filter
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![project_id], row_to_task)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(TmkprError::Database)
    }

    fn update_task(&self, id: &str, u: UpdateTask) -> TmkprResult<Task> {
        let conn = self.conn.lock().unwrap();
        let mut sets: Vec<String> = vec!["updated_at = datetime('now')".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(name) = u.name {
            binds.push(Box::new(name));
            sets.push(format!("name = ?{}", binds.len()));
        }
        if let Some(desc) = u.description {
            binds.push(Box::new(desc));
            sets.push(format!("description = ?{}", binds.len()));
        }
        if let Some(archived) = u.archived {
            binds.push(Box::new(archived as i64));
            sets.push(format!("archived = ?{}", binds.len()));
        }
        if let Some(pid) = u.project_id {
            binds.push(Box::new(pid.clone()));
            sets.push(format!("project_id = ?{}", binds.len()));
            // Reassign num_id within the destination project.
            // The task still lives in its old project at this point, so MAX is correct.
            binds.push(Box::new(pid));
            sets.push(format!(
                "num_id = (SELECT COALESCE(MAX(num_id), 0) + 1 FROM tasks WHERE project_id = ?{})",
                binds.len()
            ));
        }

        binds.push(Box::new(id.to_string()));
        let id_pos = binds.len();
        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{}",
            sets.join(", "),
            id_pos
        );
        let params_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        drop(conn);
        self.get_task(id)
    }

    fn delete_task(&self, id: &str) -> TmkprResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Entries ───────────────────────────────────────────────────────────────

    fn create_entry(&self, e: NewEntry) -> TmkprResult<Entry> {
        let conn = self.conn.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let tags_json = tags_to_json(&e.tags);
        conn.execute(
            "INSERT INTO entries
                (id, user_id, project_id, task_id, note, started_at, finished_at, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                e.user_id,
                e.project_id,
                e.task_id,
                e.note,
                e.started_at,
                e.finished_at,
                tags_json,
            ],
        )?;
        drop(conn);
        self.get_entry(&id)
    }

    fn get_entry(&self, id: &str) -> TmkprResult<Entry> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, task_id, note,
                    started_at, finished_at, tags, created_at, updated_at
             FROM entries WHERE id = ?1",
            params![id],
            row_to_entry,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => TmkprError::NotFound {
                entity: "entry",
                id: id.to_string(),
            },
            other => TmkprError::Database(other),
        })
    }

    fn get_active_entry(&self, user_id: &str) -> TmkprResult<Option<Entry>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, project_id, task_id, note,
                    started_at, finished_at, tags, created_at, updated_at
             FROM entries WHERE user_id = ?1 AND finished_at IS NULL
             ORDER BY started_at DESC LIMIT 1",
            params![user_id],
            row_to_entry,
        )
        .optional()
        .map_err(TmkprError::Database)
    }

    fn list_entries(&self, filter: &EntryFilter) -> TmkprResult<Vec<Entry>> {
        let conn = self.conn.lock().unwrap();
        let mut conditions = vec!["e.user_id = ?1".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(filter.user_id.clone())];

        if let Some(ref pid) = filter.project_id {
            binds.push(Box::new(pid.clone()));
            conditions.push(format!("e.project_id = ?{}", binds.len()));
        }
        if let Some(ref tid) = filter.task_id {
            binds.push(Box::new(tid.clone()));
            conditions.push(format!("e.task_id = ?{}", binds.len()));
        }
        if let Some(from) = filter.from {
            binds.push(Box::new(from));
            conditions.push(format!("e.started_at >= ?{}", binds.len()));
        }
        if let Some(until) = filter.until {
            binds.push(Box::new(until));
            conditions.push(format!("e.started_at <= ?{}", binds.len()));
        }
        if !filter.include_active {
            conditions.push("e.finished_at IS NOT NULL".to_string());
        }

        let where_clause = conditions.join(" AND ");
        let limit_clause = filter
            .limit
            .map(|l| format!("LIMIT {}", l))
            .unwrap_or_default();

        let sql = format!(
            "SELECT e.id, e.user_id, e.project_id, e.task_id, e.note,
                    e.started_at, e.finished_at, e.tags, e.created_at, e.updated_at
             FROM entries e WHERE {} ORDER BY e.started_at DESC {}",
            where_clause, limit_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), row_to_entry)?;
        let mut entries: Vec<Entry> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(TmkprError::Database)?;

        // Tag filtering (AND semantics — can't do this cleanly in SQL without json_each)
        if !filter.tags.is_empty() {
            entries.retain(|e| filter.tags.iter().all(|t| e.tags.contains(t)));
        }

        Ok(entries)
    }

    fn update_entry(&self, id: &str, u: UpdateEntry) -> TmkprResult<Entry> {
        let conn = self.conn.lock().unwrap();
        let mut sets: Vec<String> = vec!["updated_at = datetime('now')".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(pid) = u.project_id {
            binds.push(Box::new(pid));
            sets.push(format!("project_id = ?{}", binds.len()));
        }
        if let Some(tid) = u.task_id {
            binds.push(Box::new(tid));
            sets.push(format!("task_id = ?{}", binds.len()));
        }
        if let Some(note) = u.note {
            binds.push(Box::new(note));
            sets.push(format!("note = ?{}", binds.len()));
        }
        if let Some(started_at) = u.started_at {
            binds.push(Box::new(started_at));
            sets.push(format!("started_at = ?{}", binds.len()));
        }
        if let Some(finished_at) = u.finished_at {
            binds.push(Box::new(finished_at));
            sets.push(format!("finished_at = ?{}", binds.len()));
        }
        if let Some(tags) = u.tags {
            binds.push(Box::new(tags_to_json(&tags)));
            sets.push(format!("tags = ?{}", binds.len()));
        }

        binds.push(Box::new(id.to_string()));
        let id_pos = binds.len();
        let sql = format!(
            "UPDATE entries SET {} WHERE id = ?{}",
            sets.join(", "),
            id_pos
        );
        let params_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        drop(conn);
        self.get_entry(id)
    }

    fn delete_entry(&self, id: &str) -> TmkprResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn finish_entry(&self, user_id: &str, finished_at: DateTime<Utc>) -> TmkprResult<Entry> {
        let active = self
            .get_active_entry(user_id)?
            .ok_or(TmkprError::NotTracking)?;

        if finished_at < active.started_at {
            return Err(TmkprError::InvalidTimeRange);
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE entries SET finished_at = ?1, updated_at = datetime('now')
             WHERE id = ?2",
            params![finished_at, active.id],
        )?;
        drop(conn);
        self.get_entry(&active.id)
    }

    fn resolve_entry_id(&self, user_id: &str, prefix: &str) -> TmkprResult<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id FROM entries WHERE user_id = ?1 AND id LIKE ?2 || '%'")?;
        let matches: Vec<String> = stmt
            .query_map(params![user_id, prefix], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        match matches.len() {
            0 => Err(TmkprError::NotFound {
                entity: "entry",
                id: prefix.to_string(),
            }),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => Err(TmkprError::Conflict(format!(
                "prefix `{}` matches {} entries; use more characters",
                prefix,
                matches.len()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{entry::EntryFilter, LOCAL_USER_ID};

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    #[test]
    fn create_and_get_project() {
        let s = storage();
        let p = s
            .create_project(NewProject {
                user_id: LOCAL_USER_ID.to_string(),
                name: "myproject".to_string(),
                description: None,
                color: None,
            })
            .unwrap();
        assert_eq!(p.name, "myproject");

        let fetched = s.get_project(&p.id).unwrap();
        assert_eq!(fetched.id, p.id);
    }

    #[test]
    fn duplicate_project_is_conflict() {
        let s = storage();
        let new = || NewProject {
            user_id: LOCAL_USER_ID.to_string(),
            name: "dup".to_string(),
            description: None,
            color: None,
        };
        s.create_project(new()).unwrap();
        let err = s.create_project(new()).unwrap_err();
        assert!(matches!(err, TmkprError::Conflict(_)));
    }

    #[test]
    fn track_and_finish_entry() {
        let s = storage();
        let now = Utc::now();

        let entry = s
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: Some("testing".to_string()),
                started_at: now,
                finished_at: None,
                tags: vec![],
            })
            .unwrap();

        assert!(entry.is_active());

        let finished = s
            .finish_entry(LOCAL_USER_ID, now + chrono::Duration::hours(1))
            .unwrap();
        assert!(!finished.is_active());
    }

    #[test]
    fn no_double_finish() {
        let s = storage();
        let now = Utc::now();
        s.create_entry(NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now,
            finished_at: None,
            tags: vec![],
        })
        .unwrap();
        s.finish_entry(LOCAL_USER_ID, now + chrono::Duration::hours(1))
            .unwrap();
        let err = s
            .finish_entry(LOCAL_USER_ID, now + chrono::Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, TmkprError::NotTracking));
    }

    #[test]
    fn list_entries_filter_by_project() {
        let s = storage();
        let p = s
            .create_project(NewProject {
                user_id: LOCAL_USER_ID.to_string(),
                name: "proj".to_string(),
                description: None,
                color: None,
            })
            .unwrap();

        let now = Utc::now();
        s.create_entry(NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: Some(p.id.clone()),
            task_id: None,
            note: None,
            started_at: now,
            finished_at: Some(now + chrono::Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();
        s.create_entry(NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now,
            finished_at: Some(now + chrono::Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();

        let filter = EntryFilter {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: Some(p.id),
            include_active: true,
            ..Default::default()
        };
        let entries = s.list_entries(&filter).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn move_task_between_projects() {
        let s = storage();
        let proj_a = s.create_project(NewProject {
            user_id: LOCAL_USER_ID.to_string(),
            name: "a".to_string(),
            description: None,
            color: None,
        }).unwrap();
        let proj_b = s.create_project(NewProject {
            user_id: LOCAL_USER_ID.to_string(),
            name: "b".to_string(),
            description: None,
            color: None,
        }).unwrap();

        let task = s.create_task(NewTask {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: proj_a.id.clone(),
            name: "work".to_string(),
            description: None,
        }).unwrap();

        let moved = s.update_task(&task.id, UpdateTask {
            project_id: Some(proj_b.id.clone()),
            ..Default::default()
        }).unwrap();

        assert_eq!(moved.project_id, proj_b.id);
        assert_eq!(moved.num_id, 1);
        assert!(s.list_tasks(&proj_a.id, false).unwrap().is_empty());
        assert_eq!(s.list_tasks(&proj_b.id, false).unwrap().len(), 1);
    }
}
