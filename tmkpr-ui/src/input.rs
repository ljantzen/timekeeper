use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppMode, ModeKind, form_fields};
use crate::form::FormResult;

pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.running = false;
        return Ok(());
    }

    match app.mode.kind() {
        ModeKind::Normal => handle_normal(app, key),
        ModeKind::StartModal => handle_start_modal(app, key),
        ModeKind::EditModal => handle_edit_modal(app, key),
        ModeKind::ConfirmDelete => handle_confirm_delete(app, key),
        ModeKind::AddProject => handle_add_project(app, key),
        ModeKind::ManageProjects => handle_manage_projects(app, key),
        ModeKind::EditProject => handle_edit_project(app, key),
        ModeKind::FilterProjects => handle_filter_projects(app, key),
        ModeKind::AddTask => handle_add_task(app, key),
        ModeKind::ManageTasks => handle_manage_tasks(app, key),
        ModeKind::EditTask => handle_edit_task(app, key),
        ModeKind::Filter => handle_filter(app, key),
        ModeKind::FilterTasks => handle_filter_tasks(app, key),
        ModeKind::Comments => handle_comments(app, key),
        ModeKind::AddComment => handle_add_comment(app, key),
        ModeKind::Help => {
            app.mode = AppMode::Normal;
            Ok(())
        }
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.running = false;
        }
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('s') => {
            if app.active_entry.is_none() {
                app.open_start_modal();
            } else {
                app.status = Some(("Already tracking. Stop first with [x].".into(), true));
            }
        }
        KeyCode::Char('x') => {
            if app.active_entry.is_some() {
                if let Err(e) = app.stop_active() {
                    app.status = Some((e.to_string(), true));
                }
            } else {
                app.status = Some(("Not currently tracking.".into(), true));
            }
        }
        KeyCode::Char('e') => {
            if !app.entries.is_empty() {
                app.open_edit_modal();
            }
        }
        KeyCode::Char('d') => {
            if !app.entries.is_empty() {
                app.open_confirm_delete();
            }
        }
        KeyCode::Char('f') => {
            app.open_filter_modal();
        }
        KeyCode::Char('r') => match app.refresh() {
            Ok(()) => app.status = Some(("Refreshed.".into(), false)),
            Err(e) => app.status = Some((e.to_string(), true)),
        },
        KeyCode::Char('c') => {
            if !app.entries.is_empty() {
                if let Err(e) = app.open_comments() {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
        KeyCode::Char('C') => {
            if let Err(e) = app.open_comments_for_active() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('p') => app.open_manage_projects(),
        KeyCode::Char('t') => app.open_manage_tasks(),
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        _ => {}
    }
    Ok(())
}

fn handle_filter(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::Filter(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::Filter(form) = old {
                let project = form.fields[form_fields::filter::PROJECT].value.clone();
                let date_str = form.fields[form_fields::filter::DATE].value.clone();
                if let Err(e) = app.apply_filter(&project, &date_str) {
                    app.status = Some((e.to_string(), true));
                    app.mode = AppMode::Normal;
                }
            }
        }
    }
    Ok(())
}

fn handle_filter_tasks(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::FilterTasks(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.open_manage_tasks();
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::FilterTasks(form) = old {
                let project_name = form.fields[form_fields::filter_tasks::PROJECT].value.clone();
                let include_archived = form.fields[form_fields::filter_tasks::INCLUDE_ARCHIVED].value.to_lowercase();

                app.task_filter.hide_archived = !matches!(include_archived.as_str(), "y" | "yes");
                app.task_filter.project_id = if project_name.is_empty() {
                    None
                } else {
                    app.projects
                        .iter()
                        .find(|p| p.name == project_name)
                        .map(|p| p.id.clone())
                };

                app.open_manage_tasks();
            }
        }
    }
    Ok(())
}

fn handle_add_project(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::AddProject(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.open_manage_projects();
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddProject(form) = old {
                let name = form.fields[form_fields::add_project::NAME].value.clone();
                let description = form.fields[form_fields::add_project::DESCRIPTION].value.clone();
                let color = form.fields[form_fields::add_project::COLOR].value.clone();
                if let Err(e) = app.add_project(&name, &description, &color) {
                    app.status = Some((e.to_string(), true));
                    app.mode = AppMode::AddProject(form);
                } else {
                    app.open_manage_projects();
                }
            }
        }
    }
    Ok(())
}

fn handle_manage_projects(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next_project();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev_project();
        }
        KeyCode::Char('a') => {
            app.open_add_project_modal();
        }
        KeyCode::Char('e') => {
            if let Err(e) = app.open_edit_selected_project() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('s') => {
            app.project_sort = app.project_sort.next();
            app.open_manage_projects();
        }
        KeyCode::Char('f') => {
            app.open_project_filter_modal();
        }
        _ => {}
    }
    Ok(())
}

fn handle_filter_projects(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::FilterProjects(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.open_manage_projects();
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::FilterProjects(form) = old {
                let include_archived = form.fields[form_fields::filter_projects::INCLUDE_ARCHIVED].value.to_lowercase();
                app.project_filter.hide_archived = !matches!(include_archived.as_str(), "y" | "yes");
                app.open_manage_projects();
            }
        }
    }
    Ok(())
}

fn handle_edit_project(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::EditProject { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::ManageProjects {
                projects: app.projects.clone(),
                selected: 0,
            };
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::EditProject { project_id, form } = old {
                let name = form.fields[form_fields::edit_project::NAME].value.clone();
                let description = form.fields[form_fields::edit_project::DESCRIPTION].value.clone();
                let color = form.fields[form_fields::edit_project::COLOR].value.clone();
                if let Err(e) = app.submit_edit_project(project_id, &name, &description, &color) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_add_task(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::AddTask(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.open_manage_tasks();
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddTask(form) = old {
                let project = form.fields[form_fields::add_task::PROJECT].value.clone();
                let name = form.fields[form_fields::add_task::NAME].value.clone();
                let description = form.fields[form_fields::add_task::DESCRIPTION].value.clone();
                if let Err(e) = app.add_task(&project, &name, &description) {
                    app.status = Some((e.to_string(), true));
                    app.mode = AppMode::AddTask(form);
                } else {
                    app.open_manage_tasks();
                }
            }
        }
    }
    Ok(())
}

fn handle_manage_tasks(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next_task();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev_task();
        }
        KeyCode::Char('a') => {
            app.open_add_task_modal();
        }
        KeyCode::Char('e') => {
            if let Err(e) = app.open_edit_selected_task() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('d') => {
            if let Err(e) = app.delete_selected_task() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('s') => {
            app.task_sort = app.task_sort.next();
            app.open_manage_tasks();
        }
        KeyCode::Char('f') => {
            app.open_task_filter_modal();
        }
        _ => {}
    }
    Ok(())
}

fn handle_edit_task(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::EditTask { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::ManageTasks {
                tasks: app.tasks.clone(),
                selected: 0,
            };
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::EditTask { task_id, form } = old {
                let name = form.fields[form_fields::edit_task::NAME].value.clone();
                let description = form.fields[form_fields::edit_task::DESCRIPTION].value.clone();
                if let Err(e) = app.submit_edit_task(task_id, &name, &description) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_start_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::StartModal(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::StartModal(form) = old {
                let project = form.fields[form_fields::start_modal::PROJECT].value.clone();
                let task = form.fields[form_fields::start_modal::TASK].value.clone();
                let note = form.fields[form_fields::start_modal::NOTE].value.clone();
                let tags = form.fields[form_fields::start_modal::TAGS].value.clone();
                if let Err(e) = app.start_entry(&project, &task, &note, &tags) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_edit_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::EditModal { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::EditModal { id, form } = old {
                let project = form.fields[form_fields::edit_modal::PROJECT].value.clone();
                let task = form.fields[form_fields::edit_modal::TASK].value.clone();
                let note = form.fields[form_fields::edit_modal::NOTE].value.clone();
                let start = form.fields[form_fields::edit_modal::START].value.clone();
                let end = form.fields[form_fields::edit_modal::END].value.clone();
                let tags = form.fields[form_fields::edit_modal::TAGS].value.clone();
                if let Err(e) = app.edit_entry(&id, &project, &task, &note, &start, &end, &tags) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::ConfirmDelete { id, .. } = old {
                if let Err(e) = app.delete_entry(&id) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
        _ => {
            app.mode = AppMode::Normal;
        }
    }
    Ok(())
}

fn handle_comments(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let AppMode::Comments {
                comments, selected, ..
            } = &mut app.mode
            {
                if !comments.is_empty() {
                    *selected = (*selected + 1).min(comments.len() - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let AppMode::Comments { selected, .. } = &mut app.mode {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Char('a') => {
            app.open_add_comment();
        }
        KeyCode::Char('d') => {
            if let Err(e) = app.delete_selected_comment() {
                app.status = Some((e.to_string(), true));
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_comment(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::AddComment { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            if let Err(e) = app.cancel_add_comment() {
                app.mode = AppMode::Normal;
                app.status = Some((e.to_string(), true));
            }
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddComment { entry_id, form } = old {
                let body = form.fields[form_fields::add_comment::BODY].value.clone();
                if let Err(e) = app.submit_add_comment(entry_id, body) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}
