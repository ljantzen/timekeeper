use anyhow::Result;
use tmkpr_lib::models::entry::UpdateEntry;
use tmkpr_lib::nlp::{parse_datetime_now, TimeFormat};
use tmkpr_lib::service::EntryService;
use tmkpr_lib::storage::Storage;

use crate::cli::EditArgs;
use crate::output;
use crate::prompt;

#[cfg(test)]
mod tests {
    use super::*;
    use tmkpr_lib::models::entry::NewEntry;
    use tmkpr_lib::models::LOCAL_USER_ID;
    use tmkpr_lib::storage::sqlite::SqliteStorage;

    fn mem() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn seed_entry(storage: &dyn Storage, tags: Vec<&str>) -> String {
        storage
            .create_entry(NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: None,
                started_at: chrono::Utc::now() - chrono::Duration::hours(2),
                finished_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
                tags: tags.into_iter().map(|s| s.to_string()).collect(),
            })
            .unwrap()
            .id
    }

    fn edit_tags(
        storage: &dyn Storage,
        id: &str,
        add_tag: Vec<&str>,
        remove_tag: Vec<&str>,
    ) -> Vec<String> {
        let args = EditArgs {
            id: id.to_string(),
            project: None,
            task: None,
            note: None,
            start: None,
            end: None,
            tags: None,
            add_tag: add_tag.into_iter().map(|s| s.to_string()).collect(),
            remove_tag: remove_tag.into_iter().map(|s| s.to_string()).collect(),
        };
        run(
            args,
            storage,
            LOCAL_USER_ID,
            "%Y-%m-%d",
            TimeFormat::H24,
            false,
        )
        .unwrap();
        EntryService::new(storage, LOCAL_USER_ID)
            .get(id)
            .unwrap()
            .tags
    }

    #[test]
    fn add_tag_appends() {
        let s = mem();
        let id = seed_entry(&s, vec!["a"]);
        let tags = edit_tags(&s, &id, vec!["b"], vec![]);
        assert_eq!(tags, vec!["a", "b"]);
    }

    #[test]
    fn add_tag_deduplicates() {
        let s = mem();
        let id = seed_entry(&s, vec!["a"]);
        let tags = edit_tags(&s, &id, vec!["a"], vec![]);
        assert_eq!(tags, vec!["a"]);
    }

    #[test]
    fn remove_tag_removes() {
        let s = mem();
        let id = seed_entry(&s, vec!["a", "b"]);
        let tags = edit_tags(&s, &id, vec![], vec!["a"]);
        assert_eq!(tags, vec!["b"]);
    }

    #[test]
    fn remove_absent_tag_is_noop() {
        let s = mem();
        let id = seed_entry(&s, vec!["a"]);
        let tags = edit_tags(&s, &id, vec![], vec!["b"]);
        assert_eq!(tags, vec!["a"]);
    }

    #[test]
    fn add_and_remove_together() {
        let s = mem();
        let id = seed_entry(&s, vec!["a", "b"]);
        let tags = edit_tags(&s, &id, vec!["c"], vec!["a"]);
        assert_eq!(tags, vec!["b", "c"]);
    }
}

pub fn run(
    args: EditArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    time_fmt: TimeFormat,
    color: bool,
) -> Result<()> {
    let started_at = args
        .start
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;
    let finished_at = args
        .end
        .as_deref()
        .map(|s| parse_datetime_now(s, time_fmt))
        .transpose()?;

    // Resolve new project (name or #N) → id, prompting to create if name not found
    let (project_id, resolved_project) = match args.project.as_deref() {
        Some("") | Some("-") => (Some(None), None),
        Some(input) => {
            let p = prompt::resolve_or_create_project(storage, user_id, input)?;
            let id = p.id.clone();
            (Some(Some(id)), Some(p))
        }
        None => (None, None),
    };

    // Resolve new task (name or #N) → id, prompting to create if name not found
    let task_id: Option<Option<String>> = match args.task.as_deref() {
        Some("") | Some("-") => Some(None),
        Some(input) => {
            let pid = match &project_id {
                Some(Some(id)) => Some(id.clone()),
                _ => {
                    EntryService::new(storage, user_id)
                        .get(&args.id)?
                        .project_id
                }
            };
            match pid {
                Some(pid) => {
                    let project = match &resolved_project {
                        Some(p) => p.clone(),
                        None => storage.get_project(&pid)?,
                    };
                    let task = prompt::resolve_or_create_task(storage, user_id, &project, input)?;
                    Some(Some(task.id))
                }
                None => {
                    return Err(tmkpr_lib::error::TmkprError::Config(
                        "task requires a project; set --project first".to_string(),
                    )
                    .into())
                }
            }
        }
        None => None,
    };

    let tags = if !args.add_tag.is_empty() || !args.remove_tag.is_empty() {
        let current = EntryService::new(storage, user_id).get(&args.id)?.tags;
        let mut updated: Vec<String> = current
            .into_iter()
            .filter(|t| !args.remove_tag.contains(t))
            .collect();
        for t in args.add_tag {
            if !updated.contains(&t) {
                updated.push(t);
            }
        }
        Some(updated)
    } else {
        args.tags
    };

    let update = UpdateEntry {
        project_id,
        task_id,
        note: args.note.map(Some),
        started_at,
        finished_at: finished_at.map(Some),
        tags,
    };

    let svc = EntryService::new(storage, user_id);
    let entry = svc.update(&args.id, update)?;

    let (projects, tasks) = output::build_indexes(storage, user_id);

    println!("Updated entry {}.", output::short_id(&args.id));
    output::print_entries_table(
        std::slice::from_ref(&entry),
        &projects,
        &tasks,
        date_fmt,
        color,
    );
    Ok(())
}
