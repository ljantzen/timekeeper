use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc, Weekday};
use serde::Serialize;

use crate::error::{TmkprError, TmkprResult};
use crate::models::entry::{Entry, EntryFilter, NewEntry, UpdateEntry, NO_PROJECT, NO_TASK};
use crate::service::{ProjectService, TaskService};
use crate::storage::Storage;
use crate::util::local_midnight_utc;

#[derive(Debug, Serialize)]
pub struct ReportData {
    pub from: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub total_secs: i64,
    pub by_project: Vec<ProjectReport>,
}

#[derive(Debug, Serialize)]
pub struct ProjectReport {
    pub project_name: String,
    pub total_secs: i64,
    pub by_task: Vec<TaskReport>,
}

#[derive(Debug, Serialize)]
pub struct TaskReport {
    pub task_name: String,
    pub total_secs: i64,
    pub entry_count: usize,
}

// ── Week report types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WeekReportDay {
    pub date: NaiveDate,
    /// (project_name, secs) for every project that has time on this day.
    pub by_project: Vec<(String, i64)>,
    pub total_secs: i64,
}

#[derive(Debug, Serialize)]
pub struct WeekReport {
    pub year: i32,
    pub week: u32,
    /// All project names that appear during the week, ordered by total secs desc.
    pub projects: Vec<String>,
    /// Monday through Sunday.
    pub days: Vec<WeekReportDay>,
    pub totals_by_project: Vec<(String, i64)>,
    pub total_secs: i64,
}

// ─────────────────────────────────────────────────────────────────────────────

pub struct EntryService<'a> {
    storage: &'a dyn Storage,
    user_id: &'a str,
}

impl<'a> EntryService<'a> {
    pub fn new(storage: &'a dyn Storage, user_id: &'a str) -> Self {
        Self { storage, user_id }
    }

    fn resolve_project_task(
        &self,
        project_name: Option<&str>,
        task_name: Option<&str>,
    ) -> TmkprResult<(Option<String>, Option<String>)> {
        let project_id = match project_name {
            Some(input) => Some(
                ProjectService::new(self.storage, self.user_id)
                    .resolve(input)?
                    .id,
            ),
            None => None,
        };
        let task_id = match (task_name, &project_id) {
            (Some(input), Some(pid)) => Some(
                TaskService::new(self.storage, self.user_id)
                    .resolve(pid, input)?
                    .id,
            ),
            (Some(name), None) => {
                return Err(TmkprError::Config(format!(
                    "task `{}` requires a project (use -p)",
                    name
                )));
            }
            (None, _) => None,
        };
        Ok((project_id, task_id))
    }

    /// Start tracking. Errors if another entry is already active.
    pub fn start(
        &self,
        project_name: Option<&str>,
        task_name: Option<&str>,
        note: Option<String>,
        tags: Vec<String>,
        started_at: Option<DateTime<Utc>>,
    ) -> TmkprResult<Entry> {
        if let Some(active) = self.storage.get_active_entry(self.user_id)? {
            return Err(TmkprError::AlreadyTracking { id: active.id });
        }
        let (project_id, task_id) = self.resolve_project_task(project_name, task_name)?;
        self.storage.create_entry(NewEntry {
            user_id: self.user_id.to_string(),
            project_id,
            task_id,
            note,
            started_at: started_at.unwrap_or_else(Utc::now),
            finished_at: None,
            tags,
        })
    }

    /// Create a finished entry directly without starting/stopping.
    pub fn log(
        &self,
        project_name: Option<&str>,
        task_name: Option<&str>,
        note: Option<String>,
        tags: Vec<String>,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> TmkprResult<Entry> {
        if started_at >= finished_at {
            return Err(TmkprError::InvalidTimeRange);
        }
        let (project_id, task_id) = self.resolve_project_task(project_name, task_name)?;
        self.storage.create_entry(NewEntry {
            user_id: self.user_id.to_string(),
            project_id,
            task_id,
            note,
            started_at,
            finished_at: Some(finished_at),
            tags,
        })
    }

    /// Stop the active entry. Errors if nothing is active.
    pub fn stop(&self, finished_at: Option<DateTime<Utc>>) -> TmkprResult<Entry> {
        self.storage
            .finish_entry(self.user_id, finished_at.unwrap_or_else(Utc::now))
    }

    /// Returns the active entry and how long it has been running.
    pub fn status(&self) -> TmkprResult<Option<(Entry, Duration)>> {
        Ok(self.storage.get_active_entry(self.user_id)?.map(|e| {
            let elapsed = e.elapsed();
            (e, elapsed)
        }))
    }

    pub fn list(&self, filter: EntryFilter) -> TmkprResult<Vec<Entry>> {
        self.storage.list_entries(&filter)
    }

    pub fn get(&self, id_or_prefix: &str) -> TmkprResult<Entry> {
        let id = self.storage.resolve_entry_id(self.user_id, id_or_prefix)?;
        self.storage.get_entry(&id)
    }

    pub fn update(&self, id_or_prefix: &str, update: UpdateEntry) -> TmkprResult<Entry> {
        let id = self.storage.resolve_entry_id(self.user_id, id_or_prefix)?;
        self.storage.update_entry(&id, update)
    }

    pub fn delete(&self, id_or_prefix: &str) -> TmkprResult<()> {
        let id = self.storage.resolve_entry_id(self.user_id, id_or_prefix)?;
        self.storage.delete_entry(&id)
    }

    pub fn week_report(&self, year: i32, week: u32, work_week: bool) -> TmkprResult<WeekReport> {
        let monday = NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).ok_or_else(|| {
            TmkprError::Config(format!("invalid ISO week {week} for year {year}"))
        })?;
        let next_monday = monday + Duration::days(7);

        let from = local_midnight_utc(monday);
        let until = local_midnight_utc(next_monday);

        let entries = self.storage.list_entries(&EntryFilter {
            user_id: self.user_id.to_string(),
            from: Some(from),
            until: Some(until),
            include_active: true,
            ..Default::default()
        })?;

        let projects = self
            .storage
            .list_projects(self.user_id, true)
            .unwrap_or_default();
        let project_name_of = |id: &str| -> String {
            projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| id.to_string())
        };

        let mut week_by_project: HashMap<String, i64> = HashMap::new();
        let mut total_secs = 0i64;

        let days: Vec<WeekReportDay> = (0..7)
            .map(|offset| {
                let date = monday + Duration::days(offset);
                let day_start = local_midnight_utc(date);
                let day_end = local_midnight_utc(date + Duration::days(1));

                let mut by_project: HashMap<String, i64> = HashMap::new();
                let mut day_total = 0i64;

                for entry in entries
                    .iter()
                    .filter(|e| e.started_at >= day_start && e.started_at < day_end)
                {
                    let secs = entry.elapsed().num_seconds();
                    day_total += secs;
                    let name = entry
                        .project_id
                        .as_deref()
                        .map(project_name_of)
                        .unwrap_or_else(|| NO_PROJECT.to_string());
                    *by_project.entry(name.clone()).or_insert(0) += secs;
                    *week_by_project.entry(name).or_insert(0) += secs;
                }
                total_secs += day_total;

                let mut by_project: Vec<(String, i64)> = by_project.into_iter().collect();
                by_project.sort_by(|a, b| a.0.cmp(&b.0));

                WeekReportDay {
                    date,
                    by_project,
                    total_secs: day_total,
                }
            })
            .collect();

        // When work_week is requested, discard Sat/Sun and recompute totals from Mon–Fri only.
        let (days, totals_by_project, total_secs) = if work_week {
            let wd: Vec<WeekReportDay> = days.into_iter().take(5).collect();
            let mut wd_by_project: HashMap<String, i64> = HashMap::new();
            let mut wd_total = 0i64;
            for day in &wd {
                for (name, secs) in &day.by_project {
                    *wd_by_project.entry(name.clone()).or_insert(0) += secs;
                    wd_total += secs;
                }
            }
            let mut totals: Vec<(String, i64)> = wd_by_project.into_iter().collect();
            totals.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            (wd, totals, wd_total)
        } else {
            let mut totals: Vec<(String, i64)> = week_by_project.into_iter().collect();
            totals.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            (days, totals, total_secs)
        };

        let project_names = totals_by_project.iter().map(|(n, _)| n.clone()).collect();

        Ok(WeekReport {
            year,
            week,
            projects: project_names,
            days,
            totals_by_project,
            total_secs,
        })
    }

    pub fn report(
        &self,
        from: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
        project_name: Option<&str>,
    ) -> TmkprResult<ReportData> {
        let project_id = match project_name {
            Some(input) => Some(
                ProjectService::new(self.storage, self.user_id)
                    .resolve(input)?
                    .id,
            ),
            None => None,
        };

        let filter = EntryFilter {
            user_id: self.user_id.to_string(),
            project_id,
            from,
            until,
            include_active: false,
            ..Default::default()
        };

        let entries = self.storage.list_entries(&filter)?;

        // Pre-load all projects and tasks to resolve IDs to names efficiently.
        let projects = self
            .storage
            .list_projects(self.user_id, true)
            .unwrap_or_default();
        let all_tasks: Vec<_> = projects
            .iter()
            .flat_map(|p| self.storage.list_tasks(&p.id, true).unwrap_or_default())
            .collect();

        let project_name_of = |id: &str| -> String {
            projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| id.to_string())
        };
        let task_name_of = |id: &str| -> String {
            all_tasks
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| id.to_string())
        };

        struct TaskAccum {
            total_secs: i64,
            entry_count: usize,
        }
        struct ProjectAccum {
            total_secs: i64,
            tasks: HashMap<String, TaskAccum>,
        }

        let mut total_secs: i64 = 0;
        let mut proj_map: HashMap<String, ProjectAccum> = HashMap::new();

        for entry in &entries {
            let dur = entry.duration().unwrap_or_default().num_seconds();
            total_secs += dur;

            let proj_key = entry
                .project_id
                .as_deref()
                .map(&project_name_of)
                .unwrap_or_else(|| NO_PROJECT.to_string());
            let task_key = entry
                .task_id
                .as_deref()
                .map(&task_name_of)
                .unwrap_or_else(|| NO_TASK.to_string());

            let pa = proj_map.entry(proj_key).or_insert(ProjectAccum {
                total_secs: 0,
                tasks: HashMap::new(),
            });
            pa.total_secs += dur;
            let ta = pa.tasks.entry(task_key).or_insert(TaskAccum {
                total_secs: 0,
                entry_count: 0,
            });
            ta.total_secs += dur;
            ta.entry_count += 1;
        }

        let mut by_project: Vec<ProjectReport> = proj_map
            .into_iter()
            .map(|(project_name, pa)| {
                let mut by_task: Vec<TaskReport> = pa
                    .tasks
                    .into_iter()
                    .map(|(task_name, ta)| TaskReport {
                        task_name,
                        total_secs: ta.total_secs,
                        entry_count: ta.entry_count,
                    })
                    .collect();
                by_task.sort_by(|a, b| a.task_name.cmp(&b.task_name));
                ProjectReport {
                    project_name,
                    total_secs: pa.total_secs,
                    by_task,
                }
            })
            .collect();
        by_project.sort_by(|a, b| a.project_name.cmp(&b.project_name));

        Ok(ReportData {
            from,
            until,
            total_secs,
            by_project,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TmkprError;
    use crate::models::entry::UpdateEntry;
    use crate::models::LOCAL_USER_ID;
    use crate::service::{ProjectService, TaskService};
    use crate::storage::sqlite::SqliteStorage;

    fn storage() -> SqliteStorage {
        SqliteStorage::open_in_memory().unwrap()
    }

    fn svc(s: &dyn Storage) -> EntryService<'_> {
        EntryService::new(s, LOCAL_USER_ID)
    }

    fn setup(s: &dyn Storage) -> (String, String) {
        let proj = ProjectService::new(s, LOCAL_USER_ID)
            .add("myproj", None, None)
            .unwrap()
            .name;
        let task = TaskService::new(s, LOCAL_USER_ID)
            .add(&proj, "mytask", None)
            .unwrap()
            .name;
        (proj, task)
    }

    #[test]
    fn start_and_stop() {
        let s = storage();
        let entry = svc(&s).start(None, None, None, vec![], None).unwrap();
        assert!(entry.is_active());
        assert!(svc(&s).status().unwrap().is_some());

        let stopped = svc(&s).stop(None).unwrap();
        assert!(!stopped.is_active());
        assert!(svc(&s).status().unwrap().is_none());
    }

    #[test]
    fn start_with_project_and_task() {
        let s = storage();
        let (proj, task) = setup(&s);
        let entry = svc(&s)
            .start(
                Some(&proj),
                Some(&task),
                Some("note".into()),
                vec!["tag1".into()],
                None,
            )
            .unwrap();
        assert!(entry.project_id.is_some());
        assert!(entry.task_id.is_some());
        assert_eq!(entry.note.as_deref(), Some("note"));
        assert_eq!(entry.tags, vec!["tag1"]);
    }

    #[test]
    fn start_with_unknown_project_errors() {
        let s = storage();
        let err = svc(&s)
            .start(Some("ghost"), None, None, vec![], None)
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
    fn start_with_task_requires_project() {
        let s = storage();
        let err = svc(&s)
            .start(None, Some("sometask"), None, vec![], None)
            .unwrap_err();
        assert!(matches!(err, TmkprError::Config(_)));
    }

    #[test]
    fn cannot_start_while_already_tracking() {
        let s = storage();
        svc(&s).start(None, None, None, vec![], None).unwrap();
        let err = svc(&s).start(None, None, None, vec![], None).unwrap_err();
        assert!(matches!(err, TmkprError::AlreadyTracking { .. }));
    }

    #[test]
    fn stop_when_not_tracking_errors() {
        let s = storage();
        let err = svc(&s).stop(None).unwrap_err();
        assert!(matches!(err, TmkprError::NotTracking));
    }

    #[test]
    fn start_with_explicit_time() {
        let s = storage();
        let t = Utc::now() - Duration::hours(2);
        let entry = svc(&s).start(None, None, None, vec![], Some(t)).unwrap();
        let elapsed = entry.elapsed().num_seconds();
        assert!(elapsed >= 7199 && elapsed <= 7201);
    }

    #[test]
    fn stop_with_explicit_time() {
        let s = storage();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        svc(&s)
            .start(None, None, None, vec![], Some(start))
            .unwrap();
        let stopped = svc(&s).stop(Some(end)).unwrap();
        let dur = stopped.duration().unwrap().num_seconds();
        assert!(dur >= 3599 && dur <= 3601);
    }

    #[test]
    fn list_and_filter() {
        let s = storage();
        let (proj, _) = setup(&s);
        let proj_id = crate::service::ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;

        let now = Utc::now();
        // Entry with project
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: Some(proj_id.clone()),
            task_id: None,
            note: None,
            started_at: now - Duration::hours(2),
            finished_at: Some(now - Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();
        // Entry without project
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now - Duration::hours(3),
            finished_at: Some(now - Duration::hours(2)),
            tags: vec![],
        })
        .unwrap();

        let all = svc(&s)
            .list(EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(all.len(), 2);

        let filtered = svc(&s)
            .list(EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: Some(proj_id),
                include_active: false,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn get_by_prefix() {
        let s = storage();
        let entry = svc(&s).start(None, None, None, vec![], None).unwrap();
        svc(&s).stop(None).unwrap();
        let prefix = &entry.id[..8];
        let fetched = svc(&s).get(prefix).unwrap();
        assert_eq!(fetched.id, entry.id);
    }

    #[test]
    fn update_note() {
        let s = storage();
        svc(&s)
            .start(None, None, Some("old".into()), vec![], None)
            .unwrap();
        let entry = svc(&s).stop(None).unwrap();
        let updated = svc(&s)
            .update(
                &entry.id,
                UpdateEntry {
                    note: Some(Some("new note".into())),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.note.as_deref(), Some("new note"));
    }

    #[test]
    fn delete_entry() {
        let s = storage();
        svc(&s).start(None, None, None, vec![], None).unwrap();
        let entry = svc(&s).stop(None).unwrap();
        svc(&s).delete(&entry.id).unwrap();
        let all = svc(&s)
            .list(EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                ..Default::default()
            })
            .unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn report_totals() {
        let s = storage();
        let (proj, task) = setup(&s);
        let now = Utc::now();

        let proj_id = crate::service::ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;
        let task_id = s.get_task_by_name(&proj_id, &task).unwrap().unwrap().id;

        for _ in 0..3 {
            s.create_entry(crate::models::entry::NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: Some(proj_id.clone()),
                task_id: Some(task_id.clone()),
                note: None,
                started_at: now - Duration::hours(2),
                finished_at: Some(now - Duration::hours(1)),
                tags: vec![],
            })
            .unwrap();
        }

        let report = svc(&s).report(None, None, None).unwrap();
        assert_eq!(report.total_secs, 3 * 3600);
        assert_eq!(report.by_project.len(), 1);
        assert_eq!(report.by_project[0].project_name, proj);
        assert_eq!(report.by_project[0].by_task[0].task_name, task);
        assert_eq!(report.by_project[0].by_task[0].entry_count, 3);
    }

    #[test]
    fn report_filter_by_project() {
        let s = storage();
        let now = Utc::now();
        let (proj, _) = setup(&s);
        let proj_id = crate::service::ProjectService::new(&s, LOCAL_USER_ID)
            .get_by_name(&proj)
            .unwrap()
            .unwrap()
            .id;

        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: Some(proj_id),
            task_id: None,
            note: None,
            started_at: now - Duration::hours(1),
            finished_at: Some(now),
            tags: vec![],
        })
        .unwrap();
        // Entry with no project — should be excluded when filtering
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now - Duration::hours(2),
            finished_at: Some(now - Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();

        let report = svc(&s).report(None, None, Some(&proj)).unwrap();
        assert_eq!(report.by_project.len(), 1);
        assert_eq!(report.total_secs, 3600);
    }

    #[test]
    fn log_creates_finished_entry() {
        let s = storage();
        let (proj, task) = setup(&s);
        let start = Utc::now() - Duration::hours(2);
        let end = Utc::now() - Duration::hours(1);
        let entry = svc(&s)
            .log(
                Some(&proj),
                Some(&task),
                Some("note".into()),
                vec!["t".into()],
                start,
                end,
            )
            .unwrap();
        assert!(!entry.is_active());
        assert!(entry.project_id.is_some());
        assert!(entry.task_id.is_some());
        assert_eq!(entry.duration().unwrap().num_seconds(), 3600);
    }

    #[test]
    fn log_without_project_or_task() {
        let s = storage();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        let entry = svc(&s).log(None, None, None, vec![], start, end).unwrap();
        assert!(!entry.is_active());
        assert!(entry.project_id.is_none());
    }

    #[test]
    fn log_invalid_time_range_errors() {
        let s = storage();
        let now = Utc::now();
        let err = svc(&s)
            .log(None, None, None, vec![], now, now - Duration::seconds(1))
            .unwrap_err();
        assert!(matches!(err, TmkprError::InvalidTimeRange));
    }

    #[test]
    fn log_equal_times_errors() {
        let s = storage();
        let now = Utc::now();
        let err = svc(&s).log(None, None, None, vec![], now, now).unwrap_err();
        assert!(matches!(err, TmkprError::InvalidTimeRange));
    }

    #[test]
    fn log_task_without_project_errors() {
        let s = storage();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        let err = svc(&s)
            .log(None, Some("sometask"), None, vec![], start, end)
            .unwrap_err();
        assert!(matches!(err, TmkprError::Config(_)));
    }

    #[test]
    fn list_filters_by_time_range() {
        let s = storage();
        let now = Utc::now();
        let today_midnight = {
            use chrono::{Local, TimeZone};
            let today = chrono::Local::now().date_naive();
            Local
                .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap()
                .with_timezone(&Utc)
        };

        // Entry from yesterday — should be excluded by a today filter
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: Some("yesterday".into()),
            started_at: today_midnight - Duration::hours(2),
            finished_at: Some(today_midnight - Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();

        // Entry from today — should be included
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: Some("today".into()),
            started_at: now - Duration::hours(1),
            finished_at: Some(now),
            tags: vec![],
        })
        .unwrap();

        let results = svc(&s)
            .list(EntryFilter {
                user_id: LOCAL_USER_ID.to_string(),
                from: Some(today_midnight),
                include_active: false,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note.as_deref(), Some("today"));
    }

    #[test]
    fn log_unknown_project_errors() {
        let s = storage();
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        let err = svc(&s)
            .log(Some("ghost"), None, None, vec![], start, end)
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
    fn report_unknown_project_errors() {
        let s = storage();
        let err = svc(&s).report(None, None, Some("ghost")).unwrap_err();
        assert!(matches!(
            err,
            TmkprError::NotFound {
                entity: "project",
                ..
            }
        ));
    }

    #[test]
    fn report_with_unassigned_entries() {
        let s = storage();
        let now = Utc::now();

        // Entry with no project and no task
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now - Duration::hours(2),
            finished_at: Some(now - Duration::hours(1)),
            tags: vec![],
        })
        .unwrap();
        // Second entry in same "(no project)" bucket so the existing-bucket branch is hit
        s.create_entry(crate::models::entry::NewEntry {
            user_id: LOCAL_USER_ID.to_string(),
            project_id: None,
            task_id: None,
            note: None,
            started_at: now - Duration::hours(4),
            finished_at: Some(now - Duration::hours(3)),
            tags: vec![],
        })
        .unwrap();

        let report = svc(&s).report(None, None, None).unwrap();
        assert_eq!(report.by_project.len(), 1);
        assert_eq!(report.by_project[0].project_name, "(no project)");
        assert_eq!(report.by_project[0].by_task[0].task_name, "(no task)");
        assert_eq!(report.by_project[0].by_task[0].entry_count, 2);
        assert_eq!(report.total_secs, 7200);
    }

    #[test]
    fn update_entry_time_and_tags() {
        let s = storage();
        let now = Utc::now();
        let entry = s
            .create_entry(crate::models::entry::NewEntry {
                user_id: LOCAL_USER_ID.to_string(),
                project_id: None,
                task_id: None,
                note: None,
                started_at: now - Duration::hours(2),
                finished_at: Some(now - Duration::hours(1)),
                tags: vec![],
            })
            .unwrap();

        let new_start = now - Duration::hours(3);
        let new_end = now - Duration::minutes(30);
        let updated = svc(&s)
            .update(
                &entry.id,
                UpdateEntry {
                    started_at: Some(new_start),
                    finished_at: Some(Some(new_end)),
                    tags: Some(vec!["billable".to_string()]),
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(updated.started_at, new_start);
        assert_eq!(updated.finished_at, Some(new_end));
        assert_eq!(updated.tags, vec!["billable"]);
    }
}
