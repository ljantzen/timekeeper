use rusqlite::Connection;

use crate::error::TmkprResult;

const MIGRATION_001: &str = "
CREATE TABLE IF NOT EXISTS schema_versions (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS users (
    id           TEXT PRIMARY KEY,
    username     TEXT NOT NULL UNIQUE,
    display_name TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO users (id, username) VALUES
    ('00000000-0000-0000-0000-000000000001', 'local');

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    color       TEXT,
    archived    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, name)
);

CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    archived    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, name)
);

CREATE TABLE IF NOT EXISTS entries (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id  TEXT REFERENCES projects(id) ON DELETE SET NULL,
    task_id     TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    note        TEXT,
    started_at  TEXT NOT NULL,
    finished_at TEXT,
    tags        TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entries_user_started ON entries(user_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_project ON entries(project_id);
CREATE INDEX IF NOT EXISTS idx_entries_task ON entries(task_id);

INSERT OR IGNORE INTO schema_versions(version) VALUES (1);
";

const MIGRATION_002: &str = "
ALTER TABLE projects ADD COLUMN num_id INTEGER;
ALTER TABLE tasks    ADD COLUMN num_id INTEGER;

UPDATE projects SET num_id = (
    SELECT COUNT(*) FROM projects p2
    WHERE p2.user_id = projects.user_id
    AND (p2.created_at < projects.created_at
         OR (p2.created_at = projects.created_at AND p2.id <= projects.id))
);

UPDATE tasks SET num_id = (
    SELECT COUNT(*) FROM tasks t2
    WHERE t2.project_id = tasks.project_id
    AND (t2.created_at < tasks.created_at
         OR (t2.created_at = tasks.created_at AND t2.id <= tasks.id))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_user_num_id ON projects(user_id, num_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_project_num_id ON tasks(project_id, num_id);

INSERT OR IGNORE INTO schema_versions(version) VALUES (2);
";

const MIGRATION_003: &str = "
CREATE TABLE IF NOT EXISTS entry_comments (
    id         TEXT PRIMARY KEY,
    entry_id   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entry_comments_entry ON entry_comments(entry_id);

INSERT OR IGNORE INTO schema_versions(version) VALUES (3);
";

const MIGRATION_004: &str = "
ALTER TABLE tasks ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;

INSERT OR IGNORE INTO schema_versions(version) VALUES (4);
";

const MIGRATIONS: &[(i64, &str)] = &[
    (1, MIGRATION_001),
    (2, MIGRATION_002),
    (3, MIGRATION_003),
    (4, MIGRATION_004),
];

pub fn run_migrations(conn: &Connection) -> TmkprResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_versions (
            version    INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for (version, sql) in MIGRATIONS {
        if *version > current_version {
            conn.execute_batch(sql)?;
        }
    }

    Ok(())
}
