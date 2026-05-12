use chrono::{DateTime, Local, TimeZone, Utc};

use crate::error::{TmkprError, TmkprResult};
use crate::models::comment::{Comment, NewComment};
use crate::models::entry::EntryFilter;
use crate::storage::Storage;

pub struct CommentService<'a> {
    storage: &'a dyn Storage,
    user_id: &'a str,
}

impl<'a> CommentService<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str) -> Self {
        Self { storage, user_id }
    }

    pub fn add(&self, entry_id_or_prefix: Option<&str>, body: String) -> TmkprResult<Comment> {
        let entry_id = match entry_id_or_prefix {
            Some(prefix) => self.storage.resolve_entry_id(self.user_id, prefix)?,
            None => {
                self.storage
                    .get_active_entry(self.user_id)?
                    .ok_or(TmkprError::NotTracking)?
                    .id
            }
        };
        self.storage.create_comment(NewComment { entry_id, body })
    }

    pub fn list(&self, entry_id_or_prefix: Option<&str>) -> TmkprResult<Vec<Comment>> {
        match entry_id_or_prefix {
            Some(prefix) => {
                let id = self.storage.resolve_entry_id(self.user_id, prefix)?;
                self.storage.list_comments(&id)
            }
            None => {
                let entries = self.storage.list_entries(&EntryFilter {
                    user_id: self.user_id.to_string(),
                    from: Some(today_midnight()),
                    include_active: true,
                    ..Default::default()
                })?;
                let mut comments = Vec::new();
                for entry in entries {
                    comments.extend(self.storage.list_comments(&entry.id)?);
                }
                Ok(comments)
            }
        }
    }

    pub fn edit(&self, comment_id_prefix: &str, body: String) -> TmkprResult<Comment> {
        let id = self
            .storage
            .resolve_comment_id(self.user_id, comment_id_prefix)?;
        self.storage.update_comment(&id, body)
    }

    pub fn delete(&self, comment_id_prefix: &str) -> TmkprResult<()> {
        let id = self
            .storage
            .resolve_comment_id(self.user_id, comment_id_prefix)?;
        self.storage.delete_comment(&id)
    }
}

fn today_midnight() -> DateTime<Utc> {
    let today = Local::now().date_naive();
    Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap()
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::entry::NewEntry;
    use crate::models::LOCAL_USER_ID;
    use crate::storage::sqlite::SqliteStorage;
    use chrono::Utc;

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn svc(s: &dyn Storage) -> CommentService<'_> {
        CommentService::new(s, LOCAL_USER_ID)
    }

    fn start_entry(s: &dyn Storage) -> String {
        s.create_entry(NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: Utc::now(),
            finished_at: None,
            tags: vec![],
        })
        .unwrap()
        .id
    }

    #[test]
    fn add_to_active_entry() {
        let s = storage();
        start_entry(&s);
        let comment = svc(&s).add(None, "hello world".to_string()).unwrap();
        assert_eq!(comment.body, "hello world");
    }

    #[test]
    fn add_to_explicit_entry() {
        let s = storage();
        let entry_id = start_entry(&s);
        // finish it so it is no longer active, then add by prefix
        s.finish_entry(LOCAL_USER_ID, Utc::now() + chrono::Duration::hours(1))
            .unwrap();

        let comment = svc(&s)
            .add(Some(&entry_id), "on finished entry".to_string())
            .unwrap();
        assert_eq!(comment.entry_id, entry_id);
    }

    #[test]
    fn add_without_active_entry_errors() {
        let s = storage();
        let err = svc(&s).add(None, "oops".to_string()).unwrap_err();
        assert!(matches!(err, TmkprError::NotTracking));
    }

    #[test]
    fn list_by_entry_prefix() {
        let s = storage();
        let entry_id = start_entry(&s);
        svc(&s).add(None, "first".to_string()).unwrap();
        svc(&s).add(None, "second".to_string()).unwrap();

        let comments = svc(&s).list(Some(&entry_id)).unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].body, "first");
        assert_eq!(comments[1].body, "second");
    }

    #[test]
    fn list_no_arg_returns_todays_comments() {
        let s = storage();
        // Two entries today, one comment each.
        let e1 = start_entry(&s);
        svc(&s).add(Some(&e1), "on first".to_string()).unwrap();
        s.finish_entry(LOCAL_USER_ID, Utc::now() + chrono::Duration::hours(1))
            .unwrap();

        let e2 = start_entry(&s);
        svc(&s).add(Some(&e2), "on second".to_string()).unwrap();

        let comments = svc(&s).list(None).unwrap();
        assert_eq!(comments.len(), 2);
        assert!(comments.iter().any(|c| c.body == "on first"));
        assert!(comments.iter().any(|c| c.body == "on second"));
    }

    #[test]
    fn list_no_arg_empty_when_no_entries_today() {
        let s = storage();
        let comments = svc(&s).list(None).unwrap();
        assert!(comments.is_empty());
    }

    #[test]
    fn edit_comment() {
        let s = storage();
        start_entry(&s);
        let comment = svc(&s).add(None, "original".to_string()).unwrap();

        let updated = svc(&s).edit(&comment.id, "updated".to_string()).unwrap();
        assert_eq!(updated.body, "updated");
        assert_eq!(updated.id, comment.id);
    }

    #[test]
    fn delete_comment() {
        let s = storage();
        let entry_id = start_entry(&s);
        let comment = svc(&s).add(None, "bye".to_string()).unwrap();

        svc(&s).delete(&comment.id).unwrap();
        let remaining = svc(&s).list(Some(&entry_id)).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn resolve_comment_id_prefix() {
        let s = storage();
        start_entry(&s);
        let comment = svc(&s).add(None, "test".to_string()).unwrap();

        let prefix = &comment.id[..8];
        let updated = svc(&s).edit(prefix, "via prefix".to_string()).unwrap();
        assert_eq!(updated.body, "via prefix");
    }
}
