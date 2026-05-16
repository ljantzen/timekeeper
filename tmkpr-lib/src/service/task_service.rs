use crate::error::{TmkprError, TmkprResult};
use crate::models::task::{NewTask, Task, UpdateTask};
use crate::storage::Storage;

pub struct TaskService<'a> {
    storage: &'a dyn Storage,
    user_id: &'a str,
}

impl<'a> TaskService<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str) -> Self {
        Self { storage, user_id }
    }

    /// Resolve a CLI argument to a task within a project: tries numeric ID first, then name.
    pub fn resolve(&self, project_id: &str, input: &str) -> TmkprResult<crate::models::task::Task> {
        let found = if let Ok(n) = input.parse::<u32>() {
            self.storage.get_task_by_num_id(project_id, n)?
        } else {
            self.storage.get_task_by_name(project_id, input)?
        };
        found.ok_or_else(|| TmkprError::NotFound {
            entity: "task",
            id: input.to_string(),
        })
    }

    pub fn add(
        &self,
        project_name: &str,
        name: impl Into<String>,
        description: Option<String>,
    ) -> TmkprResult<Task> {
        let project = self
            .storage
            .get_project_by_name(self.user_id, project_name)?
            .ok_or_else(|| TmkprError::NotFound {
                entity: "project",
                id: project_name.to_string(),
            })?;

        self.storage.create_task(NewTask {
            user_id: self.user_id.to_string(),
            project_id: project.id,
            name: name.into(),
            description,
        })
    }

    pub fn list(&self, project_name: &str, include_archived: bool) -> TmkprResult<Vec<Task>> {
        let project = self
            .storage
            .get_project_by_name(self.user_id, project_name)?
            .ok_or_else(|| TmkprError::NotFound {
                entity: "project",
                id: project_name.to_string(),
            })?;

        self.storage.list_tasks(&project.id, include_archived)
    }

    pub fn edit(
        &self,
        project_id: &str,
        input: &str,
        update: crate::models::task::UpdateTask,
    ) -> TmkprResult<crate::models::task::Task> {
        let task = self.resolve(project_id, input)?;
        self.storage.update_task(&task.id, update)
    }

    /// Soft-archive by default; `hard = true` physically deletes.
    pub fn delete(&self, project_name: &str, task_name: &str, hard: bool) -> TmkprResult<()> {
        let project = self
            .storage
            .get_project_by_name(self.user_id, project_name)?
            .ok_or_else(|| TmkprError::NotFound {
                entity: "project",
                id: project_name.to_string(),
            })?;

        let task = self
            .storage
            .get_task_by_name(&project.id, task_name)?
            .ok_or_else(|| TmkprError::NotFound {
                entity: "task",
                id: task_name.to_string(),
            })?;

        if hard {
            self.storage.delete_task(&task.id)
        } else {
            self.storage.update_task(
                &task.id,
                UpdateTask {
                    archived: Some(true),
                    ..Default::default()
                },
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LOCAL_USER_ID;
    use crate::service::ProjectService;
    use crate::storage::sqlite::SqliteStorage;

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn setup_project(s: &dyn crate::storage::Storage) -> String {
        ProjectService::new(s, LOCAL_USER_ID)
            .add("proj", None, None)
            .unwrap()
            .name
    }

    #[test]
    fn add_and_list() {
        let s = storage();
        let proj = setup_project(&s);
        let svc = TaskService::new(&s, LOCAL_USER_ID);
        svc.add(&proj, "frontend", None).unwrap();
        svc.add(&proj, "backend", Some("API work".into())).unwrap();

        let tasks = svc.list(&proj, false).unwrap();
        assert_eq!(tasks.len(), 2);
        // sorted by num_id (insertion order)
        assert_eq!(tasks[0].name, "frontend");
        assert_eq!(tasks[1].name, "backend");
    }

    #[test]
    fn add_task_unknown_project() {
        let s = storage();
        let err = TaskService::new(&s, LOCAL_USER_ID)
            .add("ghost", "task", None)
            .unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }

    #[test]
    fn soft_delete_archives_task() {
        let s = storage();
        let proj = setup_project(&s);
        let svc = TaskService::new(&s, LOCAL_USER_ID);
        svc.add(&proj, "t", None).unwrap();
        svc.delete(&proj, "t", false).unwrap();

        assert!(svc.list(&proj, false).unwrap().is_empty());
        assert_eq!(svc.list(&proj, true).unwrap().len(), 1);
    }

    #[test]
    fn hard_delete_removes_task() {
        let s = storage();
        let proj = setup_project(&s);
        let svc = TaskService::new(&s, LOCAL_USER_ID);
        svc.add(&proj, "t", None).unwrap();
        svc.delete(&proj, "t", true).unwrap();

        assert!(svc.list(&proj, true).unwrap().is_empty());
    }

    #[test]
    fn delete_unknown_task_errors() {
        let s = storage();
        let proj = setup_project(&s);
        let err = TaskService::new(&s, LOCAL_USER_ID)
            .delete(&proj, "ghost", false)
            .unwrap_err();
        assert!(matches!(err, TmkprError::NotFound { entity: "task", .. }));
    }

    #[test]
    fn resolve_by_name() {
        let s = storage();
        let proj = setup_project(&s);
        let proj_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;
        TaskService::new(&s, LOCAL_USER_ID)
            .add(&proj, "mytask", None)
            .unwrap();
        let t = TaskService::new(&s, LOCAL_USER_ID)
            .resolve(&proj_id, "mytask")
            .unwrap();
        assert_eq!(t.name, "mytask");
    }

    #[test]
    fn resolve_by_num_id() {
        let s = storage();
        let proj = setup_project(&s);
        let proj_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;
        let svc = TaskService::new(&s, LOCAL_USER_ID);
        svc.add(&proj, "first", None).unwrap();
        svc.add(&proj, "second", None).unwrap();
        let t = svc.resolve(&proj_id, "2").unwrap();
        assert_eq!(t.name, "second");
    }

    #[test]
    fn resolve_unknown_task_errors() {
        let s = storage();
        let proj = setup_project(&s);
        let proj_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;
        let err = TaskService::new(&s, LOCAL_USER_ID)
            .resolve(&proj_id, "ghost")
            .unwrap_err();
        assert!(matches!(err, TmkprError::NotFound { entity: "task", .. }));
    }

    #[test]
    fn edit_name_and_description() {
        let s = storage();
        let proj = setup_project(&s);
        let proj_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;
        TaskService::new(&s, LOCAL_USER_ID)
            .add(&proj, "old", Some("desc".into()))
            .unwrap();
        let updated = TaskService::new(&s, LOCAL_USER_ID)
            .edit(
                &proj_id,
                "old",
                crate::models::task::UpdateTask {
                    name: Some("new".into()),
                    description: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.name, "new");
        assert!(updated.description.is_none());
    }

    #[test]
    fn move_task_to_another_project() {
        let s = storage();
        let proj_a = setup_project(&s);
        let proj_b = ProjectService::new(&s, LOCAL_USER_ID)
            .add("proj_b", None, None)
            .unwrap()
            .name;
        let proj_a_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj_a)
            .unwrap()
            .unwrap()
            .id;
        let proj_b_id = ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj_b)
            .unwrap()
            .unwrap()
            .id;

        TaskService::new(&s, LOCAL_USER_ID)
            .add(&proj_a, "mytask", None)
            .unwrap();

        let moved = TaskService::new(&s, LOCAL_USER_ID)
            .edit(
                &proj_a_id,
                "mytask",
                crate::models::task::UpdateTask {
                    project_id: Some(proj_b_id.clone()),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(moved.project_id, proj_b_id);
        assert_eq!(moved.num_id, 1);
        assert!(TaskService::new(&s, LOCAL_USER_ID)
            .list(&proj_a, false)
            .unwrap()
            .is_empty());
        assert_eq!(
            TaskService::new(&s, LOCAL_USER_ID)
                .list(&proj_b, false)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn add_unknown_project_errors() {
        let s = storage();
        let err = TaskService::new(&s, LOCAL_USER_ID)
            .add("ghost", "task", None)
            .unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }

    #[test]
    fn list_unknown_project_errors() {
        let s = storage();
        let err = TaskService::new(&s, LOCAL_USER_ID)
            .list("ghost", false)
            .unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }
}
