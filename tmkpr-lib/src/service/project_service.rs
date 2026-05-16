use crate::error::TmkprResult;
use crate::models::project::{NewProject, Project, UpdateProject};
use crate::storage::Storage;

pub struct ProjectService<'a> {
    storage: &'a dyn Storage,
    user_id: &'a str,
}

impl<'a> ProjectService<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str) -> Self {
        Self { storage, user_id }
    }

    pub fn add(
        &self,
        name: impl Into<String>,
        description: Option<String>,
        color: Option<String>,
    ) -> TmkprResult<Project> {
        self.storage.create_project(NewProject {
            user_id: self.user_id.to_string(),
            name: name.into(),
            description,
            color,
        })
    }

    pub fn list(&self, include_archived: bool) -> TmkprResult<Vec<Project>> {
        self.storage.list_projects(self.user_id, include_archived)
    }

    pub fn get_by_name(&self, name: &str) -> TmkprResult<Option<Project>> {
        self.storage.get_project_by_name(self.user_id, name)
    }

    /// Resolve a CLI argument to a project: tries numeric ID first, then name.
    pub fn resolve(&self, input: &str) -> TmkprResult<Project> {
        let found = if let Ok(n) = input.parse::<u32>() {
            self.storage.get_project_by_num_id(self.user_id, n)?
        } else {
            self.storage.get_project_by_name(self.user_id, input)?
        };
        found.ok_or_else(|| crate::error::TmkprError::NotFound {
            entity: "project",
            id: input.to_string(),
        })
    }

    pub fn edit(
        &self,
        input: &str,
        update: crate::models::project::UpdateProject,
    ) -> TmkprResult<Project> {
        let project = self.resolve(input)?;
        self.storage.update_project(&project.id, update)
    }

    /// Soft-archive by default; `hard = true` physically deletes.
    pub fn delete(&self, name: &str, hard: bool) -> TmkprResult<()> {
        let project = self
            .storage
            .get_project_by_name(self.user_id, name)?
            .ok_or_else(|| crate::error::TmkprError::NotFound {
                entity: "project",
                id: name.to_string(),
            })?;

        if hard {
            self.storage.delete_project(&project.id)
        } else {
            self.storage.update_project(
                &project.id,
                UpdateProject {
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
    use crate::error::TmkprError;
    use crate::models::LOCAL_USER_ID;
    use crate::storage::sqlite::SqliteStorage;

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn svc(s: &dyn Storage) -> ProjectService<'_> {
        ProjectService::new(s, LOCAL_USER_ID)
    }

    #[test]
    fn add_and_list() {
        let s = storage();
        svc(&s).add("alpha", None, None).unwrap();
        svc(&s)
            .add("beta", Some("desc".into()), Some("#ff0000".into()))
            .unwrap();
        let projects = svc(&s).list(false).unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "alpha");
        assert_eq!(projects[1].name, "beta");
        assert_eq!(projects[1].description.as_deref(), Some("desc"));
        assert_eq!(projects[1].color.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn get_by_name_found_and_missing() {
        let s = storage();
        svc(&s).add("myproj", None, None).unwrap();
        assert!(svc(&s).get_by_name("myproj").unwrap().is_some());
        assert!(svc(&s).get_by_name("nope").unwrap().is_none());
    }

    #[test]
    fn soft_delete_archives() {
        let s = storage();
        svc(&s).add("proj", None, None).unwrap();
        svc(&s).delete("proj", false).unwrap();

        let active = svc(&s).list(false).unwrap();
        assert!(active.is_empty());

        let all = svc(&s).list(true).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].archived);
    }

    #[test]
    fn hard_delete_removes() {
        let s = storage();
        svc(&s).add("proj", None, None).unwrap();
        svc(&s).delete("proj", true).unwrap();

        let all = svc(&s).list(true).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn delete_unknown_project_errors() {
        let s = storage();
        let err = svc(&s).delete("ghost", false).unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }

    #[test]
    fn resolve_by_name() {
        let s = storage();
        svc(&s).add("alpha", None, None).unwrap();
        let p = svc(&s).resolve("alpha").unwrap();
        assert_eq!(p.name, "alpha");
    }

    #[test]
    fn resolve_by_num_id() {
        let s = storage();
        svc(&s).add("first", None, None).unwrap();
        svc(&s).add("second", None, None).unwrap();
        let p = svc(&s).resolve("2").unwrap();
        assert_eq!(p.name, "second");
    }

    #[test]
    fn resolve_unknown_errors() {
        let s = storage();
        let err = svc(&s).resolve("ghost").unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }

    #[test]
    fn edit_name_and_description() {
        let s = storage();
        svc(&s).add("old", Some("desc".into()), None).unwrap();
        let updated = svc(&s)
            .edit(
                "old",
                crate::models::project::UpdateProject {
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
    fn edit_color() {
        let s = storage();
        svc(&s).add("proj", None, None).unwrap();
        let updated = svc(&s)
            .edit(
                "proj",
                crate::models::project::UpdateProject {
                    color: Some(Some("#abcdef".into())),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.color.as_deref(), Some("#abcdef"));
        let cleared = svc(&s)
            .edit(
                "proj",
                crate::models::project::UpdateProject {
                    color: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(cleared.color.is_none());
    }
}
