use crate::config::{Config, ObsidianConfig};
use crate::error::TmkprResult;
use crate::models::comment::Comment;
use crate::models::entry::Entry;
use crate::models::project::Project;
use crate::models::task::Task;
use obsidian_logging::commands::add::handle_plain_entry;

#[derive(Debug, Clone, Copy)]
pub enum ActivityAction {
    Started,
    Stopped,
    Edited,
    Completed,
    Merged,
    Deleted,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskAction {
    Created,
    Updated,
    Completed,
    Deleted,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectAction {
    Created,
    Updated,
    Deleted,
}

pub struct ObsidianLogger {
    config: ObsidianConfig,
}

impl ObsidianLogger {
    pub fn new(config: ObsidianConfig) -> Self {
        Self { config }
    }

    pub fn from_config(config: &Config) -> Self {
        Self::new(config.obsidian.clone())
    }

    /// Log an activity (entry) to Obsidian. Only logs if Obsidian integration is enabled.
    pub fn log_activity(
        &self,
        entry: &Entry,
        project_name: Option<&str>,
        task_name: Option<&str>,
        action: ActivityAction,
    ) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format_entry_message(entry, project_name, task_name, action);
        let category = self.config.activity_category.as_deref();

        log_to_obsidian(vault_dir, &message, category)?;

        Ok(())
    }

    /// Log a task action to Obsidian. Only logs if Obsidian integration is enabled.
    pub fn log_task(
        &self,
        project_name: &str,
        task_name: &str,
        action: TaskAction,
    ) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format_task_message(project_name, task_name, action);
        let category = self.config.activity_category.as_deref();

        log_to_obsidian(vault_dir, &message, category)?;

        Ok(())
    }

    /// Log a project action to Obsidian. Only logs if Obsidian integration is enabled.
    pub fn log_project(
        &self,
        project_name: &str,
        action: ProjectAction,
    ) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format_project_message(project_name, action);
        let category = self.config.activity_category.as_deref();

        log_to_obsidian(vault_dir, &message, category)?;

        Ok(())
    }

    /// Log a comment to Obsidian. Only logs if Obsidian integration is enabled.
    pub fn log_comment(&self, comment: &Comment) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format!("Comment: {}", &comment.body);
        let category = self.config.comment_category.as_deref();

        log_to_obsidian(vault_dir, &message, category)?;

        Ok(())
    }

    /// Log a project creation to Obsidian.
    pub fn log_project_created(&self, project: &Project) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = if let Some(desc) = &project.description {
            format!("[CREATED] {} - {}", project.name, desc)
        } else {
            format!("[CREATED] {}", project.name)
        };

        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }

    /// Log a project update to Obsidian.
    pub fn log_project_updated(&self, project: &Project) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format!("[UPDATED] {}", project.name);
        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }

    /// Log a project deletion to Obsidian.
    pub fn log_project_deleted(&self, project_name: &str) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format!("[DELETED] {}", project_name);
        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }

    /// Log a task creation to Obsidian.
    pub fn log_task_created(&self, project_name: &str, task: &Task) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = if let Some(desc) = &task.description {
            format!("[CREATED] {} / {} - {}", project_name, task.name, desc)
        } else {
            format!("[CREATED] {} / {}", project_name, task.name)
        };

        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }

    /// Log a task update to Obsidian.
    pub fn log_task_updated(&self, project_name: &str, task: &Task) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let action = if task.completed { "COMPLETED" } else { "UPDATED" };
        let message = format!("[{}] {} / {}", action, project_name, task.name);
        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }

    /// Log a task deletion to Obsidian.
    pub fn log_task_deleted(&self, project_name: &str, task_name: &str) -> TmkprResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let vault_dir = match &self.config.vault_dir {
            Some(dir) => dir,
            None => return Ok(()),
        };

        if !vault_dir.exists() {
            return Ok(());
        }

        let message = format!("[DELETED] {} / {}", project_name, task_name);
        log_to_obsidian(vault_dir, &message, None)?;
        Ok(())
    }
}

/// Convenience function to log an activity from anywhere.
pub fn log_activity_to_obsidian(
    config: &Config,
    entry: &Entry,
    project_name: Option<&str>,
    task_name: Option<&str>,
    action: ActivityAction,
) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_activity(entry, project_name, task_name, action)
}

/// Convenience function to log a task action from anywhere.
pub fn log_task_to_obsidian(
    config: &Config,
    project_name: &str,
    task_name: &str,
    action: TaskAction,
) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_task(project_name, task_name, action)
}

/// Convenience function to log a project action from anywhere.
pub fn log_project_to_obsidian(
    config: &Config,
    project_name: &str,
    action: ProjectAction,
) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_project(project_name, action)
}

/// Convenience function to log a comment from anywhere.
pub fn log_comment_to_obsidian(config: &Config, comment: &Comment) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_comment(comment)
}

/// Convenience function to log project creation.
pub fn log_project_created(config: &Config, project: &Project) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_project_created(project)
}

/// Convenience function to log project update.
pub fn log_project_updated(config: &Config, project: &Project) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_project_updated(project)
}

/// Convenience function to log project deletion.
pub fn log_project_deleted(config: &Config, project_name: &str) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_project_deleted(project_name)
}

/// Convenience function to log task creation.
pub fn log_task_created(config: &Config, project_name: &str, task: &Task) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_task_created(project_name, task)
}

/// Convenience function to log task update.
pub fn log_task_updated(config: &Config, project_name: &str, task: &Task) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_task_updated(project_name, task)
}

/// Convenience function to log task deletion.
pub fn log_task_deleted(config: &Config, project_name: &str, task_name: &str) -> TmkprResult<()> {
    ObsidianLogger::from_config(config).log_task_deleted(project_name, task_name)
}

fn format_task_message(project_name: &str, task_name: &str, action: TaskAction) -> String {
    let action_str = match action {
        TaskAction::Created => "CREATED",
        TaskAction::Updated => "UPDATED",
        TaskAction::Completed => "COMPLETED",
        TaskAction::Deleted => "DELETED",
    };

    format!("[{}] {} / {}", action_str, project_name, task_name)
}

fn format_project_message(project_name: &str, action: ProjectAction) -> String {
    let action_str = match action {
        ProjectAction::Created => "CREATED",
        ProjectAction::Updated => "UPDATED",
        ProjectAction::Deleted => "DELETED",
    };

    format!("[{}] {}", action_str, project_name)
}

fn format_entry_message(
    entry: &Entry,
    project_name: Option<&str>,
    task_name: Option<&str>,
    action: ActivityAction,
) -> String {
    let action_str = match action {
        ActivityAction::Started => "STARTED",
        ActivityAction::Stopped => "STOPPED",
        ActivityAction::Edited => "EDITED",
        ActivityAction::Completed => "COMPLETED",
        ActivityAction::Merged => "MERGED",
        ActivityAction::Deleted => "DELETED",
    };

    let duration = entry
        .duration()
        .map(|d| {
            let mins = d.num_minutes();
            if mins >= 60 {
                let hours = mins / 60;
                let remainder = mins % 60;
                if remainder == 0 {
                    format!("{}h", hours)
                } else {
                    format!("{}h {}m", hours, remainder)
                }
            } else {
                format!("{}m", mins)
            }
        })
        .unwrap_or_else(|| "active".to_string());

    let activity_name = match (project_name, task_name) {
        (Some(proj), Some(task)) => {
            if let Some(note) = &entry.note {
                format!("{} / {} - {}", proj, task, note)
            } else {
                format!("{} / {}", proj, task)
            }
        }
        (Some(proj), None) => {
            if let Some(note) = &entry.note {
                format!("{} / {}", proj, note)
            } else {
                proj.to_string()
            }
        }
        (None, Some(task)) => {
            if let Some(note) = &entry.note {
                format!("{} - {}", task, note)
            } else {
                task.to_string()
            }
        }
        (None, None) => {
            if let Some(note) = &entry.note {
                note.to_string()
            } else {
                "Activity".to_string()
            }
        }
    };

    format!("[{}] {} ({})", action_str, activity_name, duration)
}

fn log_to_obsidian(
    vault_dir: &std::path::Path,
    message: &str,
    category: Option<&str>,
) -> TmkprResult<()> {
    let mut config = obsidian_logging::Config::initialize();
    config.vault = vault_dir.to_string_lossy().to_string();

    handle_plain_entry(
        message.to_string(),
        std::iter::empty(),
        &config,
        true,
        category,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn logger_disabled_when_not_enabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let entry = Entry {
            id: "test-id".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: Some("Test note".to_string()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = logger.log_activity(&entry, None, None, ActivityAction::Started);
        assert!(result.is_ok(), "Should return Ok even when disabled");
    }

    #[test]
    fn logger_returns_ok_when_vault_dir_none() {
        let config = ObsidianConfig {
            enabled: true,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let entry = Entry {
            id: "test-id".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: Some("Test note".to_string()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = logger.log_activity(&entry, None, None, ActivityAction::Started);
        assert!(result.is_ok(), "Should return Ok when vault_dir is None");
    }

    #[test]
    fn format_entry_message_with_project_and_note() {
        let entry = Entry {
            id: "test".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: Some("Review PR".to_string()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now() + chrono::Duration::minutes(30)),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let message = format_entry_message(&entry, Some("Project A"), None, ActivityAction::Started);
        assert!(message.contains("[STARTED]"));
        assert!(message.contains("Project A / Review PR"));
        assert!(message.contains("30m"));
    }

    #[test]
    fn format_entry_message_without_project() {
        let entry = Entry {
            id: "test".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: Some("Coding".to_string()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now() + chrono::Duration::hours(1)),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let message = format_entry_message(&entry, None, None, ActivityAction::Stopped);
        assert!(message.contains("[STOPPED]"));
        assert!(message.contains("Coding"));
        assert!(message.contains("1h"));
    }

    #[test]
    fn format_entry_message_with_hours_and_minutes() {
        let entry = Entry {
            id: "test".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: Utc::now(),
            finished_at: Some(Utc::now() + chrono::Duration::minutes(90)),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let message = format_entry_message(&entry, Some("Work"), None, ActivityAction::Edited);
        assert!(message.contains("[EDITED]"));
        assert!(message.contains("1h 30m"));
    }

    #[test]
    fn format_entry_message_without_note() {
        let entry = Entry {
            id: "test".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: Utc::now(),
            finished_at: Some(Utc::now() + chrono::Duration::minutes(15)),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let message = format_entry_message(&entry, Some("Project X"), None, ActivityAction::Started);
        assert!(message.contains("[STARTED]"));
        assert!(message.contains("Project X"));
        assert!(message.contains("15m"));
    }

    #[test]
    fn log_comment_returns_ok_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let comment = Comment {
            id: "c1".to_string(),
            entry_id: "e1".to_string(),
            body: "Test comment".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = logger.log_comment(&comment);
        assert!(result.is_ok());
    }

    #[test]
    fn logger_from_config_creates_logger() {
        let mut config = Config::default();
        config.obsidian.enabled = true;
        config.obsidian.activity_category = Some("work".to_string());

        let logger = ObsidianLogger::from_config(&config);
        assert!(logger.config.enabled);
        assert_eq!(logger.config.activity_category, Some("work".to_string()));
    }

    #[test]
    fn log_project_created_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let project = Project {
            id: "p1".to_string(),
            user_id: "user1".to_string(),
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            color: None,
            archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            num_id: 1,
        };

        let result = logger.log_project_created(&project);
        assert!(result.is_ok());
    }

    #[test]
    fn log_task_created_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let task = Task {
            id: "t1".to_string(),
            user_id: "user1".to_string(),
            project_id: "p1".to_string(),
            name: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            archived: false,
            completed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            num_id: 1,
        };

        let result = logger.log_task_created("Test Project", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn log_project_deleted_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let result = logger.log_project_deleted("Test Project");
        assert!(result.is_ok());
    }

    #[test]
    fn log_task_deleted_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let result = logger.log_task_deleted("Test Project", "Test Task");
        assert!(result.is_ok());
    }

    #[test]
    fn log_task_updated_when_disabled() {
        let config = ObsidianConfig {
            enabled: false,
            vault_dir: None,
            activity_category: None,
            comment_category: None,
        };
        let logger = ObsidianLogger::new(config);

        let task = Task {
            id: "t1".to_string(),
            user_id: "user1".to_string(),
            project_id: "p1".to_string(),
            name: "Test Task".to_string(),
            description: None,
            archived: false,
            completed: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            num_id: 1,
        };

        let result = logger.log_task_updated("Test Project", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn format_entry_message_with_project_and_task() {
        let entry = Entry {
            id: "test".to_string(),
            user_id: "user1".to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: Utc::now(),
            finished_at: Some(Utc::now() + chrono::Duration::minutes(45)),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let message = format_entry_message(&entry, Some("teamaktiviteter"), Some("standup"), ActivityAction::Started);
        assert!(message.contains("[STARTED]"));
        assert!(message.contains("teamaktiviteter / standup"));
        assert!(message.contains("45m"));
    }
}
