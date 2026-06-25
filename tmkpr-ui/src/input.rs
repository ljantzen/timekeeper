use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{form_fields, App, AppMode, ModeKind};
use crate::form::FormResult;

pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.running = false;
        return Ok(());
    }

    match app.mode.kind() {
        ModeKind::Normal => handle_normal(app, key),
        ModeKind::Command => handle_command(app, key),
        ModeKind::StartModal => handle_start_modal(app, key),
        ModeKind::EditModal => handle_edit_modal(app, key),
        ModeKind::EditEventModal => handle_edit_event_modal(app, key),
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
        ModeKind::EditComment => handle_edit_comment(app, key),
        ModeKind::ConfirmCreate => handle_confirm_create(app, key),
        ModeKind::ConfirmDeleteProject => handle_confirm_delete_project(app, key),
        ModeKind::AddManualEntry => handle_add_manual_entry(app, key),
        ModeKind::AddEventModal => handle_add_event_modal(app, key),
        ModeKind::Settings => handle_settings(app, key),
        ModeKind::Help => handle_help(app, key),
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
        KeyCode::Char('S') => {
            if app.active_entry.is_none() {
                app.open_start_modal_from_selected();
            } else {
                app.status = Some(("Already tracking. Stop first with [x].".into(), true));
            }
        }
        KeyCode::Char('n') => {
            if app.active_entry.is_none() {
                app.open_add_manual_entry_modal();
            } else {
                app.status = Some(("Already tracking. Stop first with [x].".into(), true));
            }
        }
        KeyCode::Char('v') => {
            app.open_add_event_modal();
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
        KeyCode::Char('e') if !app.entries.is_empty() => {
            app.open_edit_modal();
        }
        KeyCode::Char('E') => {
            app.open_edit_active_modal();
        }
        KeyCode::Char('d') if !app.entries.is_empty() => {
            app.open_confirm_delete();
        }
        KeyCode::Char('f') => {
            app.open_filter_modal();
        }
        KeyCode::Char('o') => {
            app.entry_sort = app.entry_sort.next();
            if let Err(e) = app.refresh() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('T') => {
            if let Err(e) = app.apply_filter("", "today") {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('Y') => {
            if let Err(e) = app.apply_filter("", "yesterday") {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('P') => {
            if let Err(e) = app.filter_prev_week() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('N') => {
            if let Err(e) = app.filter_next_week() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('r') => match app.refresh() {
            Ok(()) => app.status = Some(("Refreshed.".into(), false)),
            Err(e) => app.status = Some((e.to_string(), true)),
        },
        KeyCode::Char('g') if !app.entries.is_empty() => {
            if let Err(e) = app.fill_gaps() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('G') => {
            if let Err(e) = app.fill_gaps_active() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('m') if !app.entries.is_empty() => {
            if let Err(e) = app.merge_with_next() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('c') if !app.entries.is_empty() => {
            if let Err(e) = app.open_comments() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('C') => {
            if let Err(e) = app.open_comments_for_active() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('p') => app.open_manage_projects(),
        KeyCode::Char('t') => app.open_manage_tasks(),
        KeyCode::Char('<') => {
            if let Err(e) = app.prev_week() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('>') => {
            if let Err(e) = app.next_week() {
                app.status = Some((e.to_string(), true));
            }
        }
        KeyCode::Char('i') => {
            app.open_settings();
        }
        KeyCode::Char('?') => {
            app.mode = AppMode::Help { scroll: 0 };
        }
        KeyCode::Char(':') => {
            app.enter_command_mode();
        }
        _ => {}
    }
    Ok(())
}

fn handle_command(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char(c) => app.command_push(c),
        KeyCode::Backspace => {
            if app.command_buf().is_empty() {
                app.command_cancel();
            } else {
                app.command_pop();
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            let forward = !key.modifiers.contains(KeyModifiers::SHIFT);
            app.command_tab(forward);
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.command_tab(false);
        }
        KeyCode::Enter => {
            app.execute_command()?;
        }
        KeyCode::Esc => {
            app.command_cancel();
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
                let project_name = form.fields[form_fields::filter_tasks::PROJECT]
                    .value
                    .clone();
                app.task_filter.hide_archived =
                    !form.fields[form_fields::filter_tasks::INCLUDE_ARCHIVED].is_on();
                app.task_filter.hide_completed =
                    !form.fields[form_fields::filter_tasks::SHOW_COMPLETED].is_on();
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
                let description = form.fields[form_fields::add_project::DESCRIPTION]
                    .value
                    .clone();
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
        KeyCode::Char('d') => {
            if let Err(e) = app.open_confirm_delete_project() {
                app.status = Some((e.to_string(), true));
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_delete_project(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::ConfirmDeleteProject { id, name } = old {
                if let Err(e) = app.delete_project(&id, &name) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
        _ => {
            app.open_manage_projects();
        }
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
                app.project_filter.hide_archived =
                    !form.fields[form_fields::filter_projects::INCLUDE_ARCHIVED].is_on();
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
                let description = form.fields[form_fields::edit_project::DESCRIPTION]
                    .value
                    .clone();
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
                let description = form.fields[form_fields::add_task::DESCRIPTION]
                    .value
                    .clone();
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
        KeyCode::Char('c') => {
            if let Err(e) = app.toggle_complete_selected_task() {
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
                let description = form.fields[form_fields::edit_task::DESCRIPTION]
                    .value
                    .clone();
                if let Err(e) = app.submit_edit_task(task_id, &name, &description) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_start_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let prev_project = if let AppMode::StartModal(form) = &app.mode {
        form.fields[form_fields::start_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    let result = match &mut app.mode {
        AppMode::StartModal(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    let project_name = if let AppMode::StartModal(form) = &app.mode {
        form.fields[form_fields::start_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    if project_name != prev_project {
        if let AppMode::StartModal(form) = &mut app.mode {
            let original_project = form.fields[form_fields::start_modal::PROJECT].original_value.clone();
            let original_task = form.fields[form_fields::start_modal::TASK].original_value.clone();
            let task = &mut form.fields[form_fields::start_modal::TASK];
            task.value = if project_name == original_project { original_task } else { String::new() };
            task.cursor = task.value.len();
            task.ac_index = None;
        }
    }

    // Update task completions based on selected project
    if !project_name.is_empty() {
        let tasks = app.task_names_for_project(&project_name);
        let task_colors = app.task_colors_for_project(&project_name);
        if let AppMode::StartModal(form) = &mut app.mode {
            form.fields[form_fields::start_modal::TASK].completions = tasks;
            form.fields[form_fields::start_modal::TASK].completion_colors = task_colors;
        }
    }

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

                let create_project = !project.is_empty()
                    && !app
                        .projects
                        .iter()
                        .any(|p| p.name == project && !p.archived);
                let create_task = !task.is_empty()
                    && !project.is_empty()
                    && (create_project || !app.task_names_for_project(&project).contains(&task));

                if create_project || create_task {
                    app.mode = AppMode::ConfirmCreate {
                        project,
                        task,
                        note,
                        tags,
                        create_project,
                        create_task,
                    };
                } else if let Err(e) = app.start_entry(&project, &task, &note, &tags) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_confirm_create(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::ConfirmCreate {
                project,
                task,
                note,
                tags,
                create_project,
                create_task,
            } = old
            {
                if let Err(e) = app.create_missing_and_start(
                    &project,
                    &task,
                    &note,
                    &tags,
                    create_project,
                    create_task,
                ) {
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

fn handle_edit_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let prev_project = if let AppMode::EditModal { form, .. } = &app.mode {
        form.fields[form_fields::edit_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    let result = match &mut app.mode {
        AppMode::EditModal { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    let project_name = if let AppMode::EditModal { form, .. } = &app.mode {
        form.fields[form_fields::edit_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    if project_name != prev_project {
        if let AppMode::EditModal { form, .. } = &mut app.mode {
            let original_project = form.fields[form_fields::edit_modal::PROJECT].original_value.clone();
            let original_task = form.fields[form_fields::edit_modal::TASK].original_value.clone();
            let task = &mut form.fields[form_fields::edit_modal::TASK];
            task.value = if project_name == original_project { original_task } else { String::new() };
            task.cursor = task.value.len();
            task.ac_index = None;
        }
    }

    // Update task completions based on selected project
    if !project_name.is_empty() {
        let tasks = app.task_names_for_project(&project_name);
        let task_colors = app.task_colors_for_project(&project_name);
        if let AppMode::EditModal { form, .. } = &mut app.mode {
            form.fields[form_fields::edit_modal::TASK].completions = tasks;
            form.fields[form_fields::edit_modal::TASK].completion_colors = task_colors;
        }
    }

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

fn handle_edit_event_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let prev_project = if let AppMode::EditEventModal { form, .. } = &app.mode {
        form.fields[form_fields::edit_event_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    let result = match &mut app.mode {
        AppMode::EditEventModal { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    let project_name = if let AppMode::EditEventModal { form, .. } = &app.mode {
        form.fields[form_fields::edit_event_modal::PROJECT].value.clone()
    } else {
        String::new()
    };

    if project_name != prev_project {
        if let AppMode::EditEventModal { form, .. } = &mut app.mode {
            let original_project = form.fields[form_fields::edit_event_modal::PROJECT].original_value.clone();
            let original_task = form.fields[form_fields::edit_event_modal::TASK].original_value.clone();
            let task = &mut form.fields[form_fields::edit_event_modal::TASK];
            task.value = if project_name == original_project { original_task } else { String::new() };
            task.cursor = task.value.len();
            task.ac_index = None;
        }
    }

    if !project_name.is_empty() {
        let tasks = app.task_names_for_project(&project_name);
        let task_colors = app.task_colors_for_project(&project_name);
        if let AppMode::EditEventModal { form, .. } = &mut app.mode {
            form.fields[form_fields::edit_event_modal::TASK].completions = tasks;
            form.fields[form_fields::edit_event_modal::TASK].completion_colors = task_colors;
        }
    }

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::EditEventModal { id, form } = old {
                let project = form.fields[form_fields::edit_event_modal::PROJECT]
                    .value
                    .clone();
                let task = form.fields[form_fields::edit_event_modal::TASK]
                    .value
                    .clone();
                let note = form.fields[form_fields::edit_event_modal::NOTE]
                    .value
                    .clone();
                let time = form.fields[form_fields::edit_event_modal::TIME]
                    .value
                    .clone();
                let tags = form.fields[form_fields::edit_event_modal::TAGS]
                    .value
                    .clone();
                if let Err(e) = app.edit_event_entry(&id, &project, &task, &note, &time, &tags) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_add_event_modal(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::AddEventModal(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    let project_name = if let AppMode::AddEventModal(form) = &app.mode {
        form.fields[form_fields::add_event_modal::PROJECT]
            .value
            .clone()
    } else {
        String::new()
    };

    if !project_name.is_empty() {
        let tasks = app.task_names_for_project(&project_name);
        let task_colors = app.task_colors_for_project(&project_name);
        if let AppMode::AddEventModal(form) = &mut app.mode {
            form.fields[form_fields::add_event_modal::TASK].completions = tasks;
            form.fields[form_fields::add_event_modal::TASK].completion_colors = task_colors;
        }
    }

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddEventModal(form) = old {
                let project = form.fields[form_fields::add_event_modal::PROJECT]
                    .value
                    .clone();
                let task = form.fields[form_fields::add_event_modal::TASK]
                    .value
                    .clone();
                let note = form.fields[form_fields::add_event_modal::NOTE]
                    .value
                    .clone();
                let time = form.fields[form_fields::add_event_modal::TIME]
                    .value
                    .clone();
                let tags = form.fields[form_fields::add_event_modal::TAGS]
                    .value
                    .clone();
                if let Err(e) = app.add_event_entry(&project, &task, &note, &time, &tags) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_add_manual_entry(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let prev_project = if let AppMode::AddManualEntry(form) = &app.mode {
        form.fields[form_fields::add_manual_entry::PROJECT].value.clone()
    } else {
        String::new()
    };

    let result = match &mut app.mode {
        AppMode::AddManualEntry(form) => form.handle_key(key),
        _ => return Ok(()),
    };

    let project_name = if let AppMode::AddManualEntry(form) = &app.mode {
        form.fields[form_fields::add_manual_entry::PROJECT].value.clone()
    } else {
        String::new()
    };

    if project_name != prev_project {
        if let AppMode::AddManualEntry(form) = &mut app.mode {
            let original_project = form.fields[form_fields::add_manual_entry::PROJECT].original_value.clone();
            let original_task = form.fields[form_fields::add_manual_entry::TASK].original_value.clone();
            let task = &mut form.fields[form_fields::add_manual_entry::TASK];
            task.value = if project_name == original_project { original_task } else { String::new() };
            task.cursor = task.value.len();
            task.ac_index = None;
        }
    }

    // Update task completions based on selected project
    if !project_name.is_empty() {
        let tasks = app.task_names_for_project(&project_name);
        let task_colors = app.task_colors_for_project(&project_name);
        if let AppMode::AddManualEntry(form) = &mut app.mode {
            form.fields[form_fields::add_manual_entry::TASK].completions = tasks;
            form.fields[form_fields::add_manual_entry::TASK].completion_colors = task_colors;
        }
    }

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddManualEntry(form) = old {
                let project = form.fields[form_fields::add_manual_entry::PROJECT]
                    .value
                    .clone();
                let task = form.fields[form_fields::add_manual_entry::TASK]
                    .value
                    .clone();
                let note = form.fields[form_fields::add_manual_entry::NOTE]
                    .value
                    .clone();
                let start = form.fields[form_fields::add_manual_entry::START]
                    .value
                    .clone();
                let end = form.fields[form_fields::add_manual_entry::END]
                    .value
                    .clone();
                let tags = form.fields[form_fields::add_manual_entry::TAGS]
                    .value
                    .clone();
                let snap_to_existing =
                    form.fields[form_fields::add_manual_entry::SNAP_TO_EXISTING].is_on();

                let create_project = !project.is_empty()
                    && !app
                        .projects
                        .iter()
                        .any(|p| p.name == project && !p.archived);
                let create_task = !task.is_empty()
                    && !project.is_empty()
                    && (create_project || !app.task_names_for_project(&project).contains(&task));

                if create_project || create_task {
                    app.mode = AppMode::ConfirmCreate {
                        project,
                        task,
                        note,
                        tags,
                        create_project,
                        create_task,
                    };
                } else if let Err(e) = app.add_manual_entry(
                    &project,
                    &task,
                    &note,
                    &start,
                    &end,
                    &tags,
                    snap_to_existing,
                ) {
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
        KeyCode::Char('e') => {
            if let Err(e) = app.open_edit_comment() {
                app.status = Some((e.to_string(), true));
            }
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

fn handle_edit_comment(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let result = match &mut app.mode {
        AppMode::EditComment { form, .. } => form.handle_key(key),
        _ => return Ok(()),
    };

    match result {
        FormResult::None => {}
        FormResult::Cancel => {
            if let Err(e) = app.cancel_edit_comment() {
                app.mode = AppMode::Normal;
                app.status = Some((e.to_string(), true));
            }
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::EditComment {
                entry_id,
                comment_id,
                form,
            } = old
            {
                let body = form.fields[form_fields::edit_comment::BODY].value.clone();
                if let Err(e) = app.submit_edit_comment(comment_id, body) {
                    app.status = Some((e.to_string(), true));
                } else if let Err(e) = app.refresh_comments_mode(entry_id, 0) {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
    }
    Ok(())
}

fn handle_help(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if let AppMode::Help { scroll } = &mut app.mode {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => *scroll = scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::PageDown => *scroll = scroll.saturating_add(10),
            KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
            _ => app.mode = AppMode::Normal,
        }
    }
    Ok(())
}

fn handle_settings(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    // Text-editing sub-mode: intercept all keys before normal navigation.
    if let AppMode::Settings {
        cursor,
        text_editing,
        obs_vault,
        obs_activity,
        obs_comment,
        ..
    } = &mut app.mode
    {
        if *text_editing {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    *text_editing = false;
                }
                KeyCode::Char(c)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    match cursor {
                        4 => obs_vault.push(c),
                        5 => obs_activity.push(c),
                        6 => obs_comment.push(c),
                        _ => {}
                    }
                }
                KeyCode::Backspace => match cursor {
                    4 => {
                        obs_vault.pop();
                    }
                    5 => {
                        obs_activity.pop();
                    }
                    6 => {
                        obs_comment.pop();
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
        }
    }

    match key.code {
        KeyCode::Esc => {
            // Restore the pre-settings theme (in case of live preview).
            let prev = app.theme_name.clone();
            let themes = app.themes.clone();
            app.theme = crate::theme::Theme::resolve(&prev, &themes);
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let AppMode::Settings { cursor, .. } = &mut app.mode {
                *cursor = (*cursor + 1) % 7;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let AppMode::Settings { cursor, .. } = &mut app.mode {
                *cursor = (*cursor + 6) % 7;
            }
        }
        KeyCode::Left | KeyCode::Char('h') => settings_adjust(app, -1),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => settings_adjust(app, 1),
        KeyCode::Enter => {
            let is_text_row = matches!(&app.mode, AppMode::Settings { cursor, .. } if *cursor >= 4);
            if is_text_row {
                if let AppMode::Settings { text_editing, .. } = &mut app.mode {
                    *text_editing = true;
                }
            } else {
                if let Err(e) = app.settings_save() {
                    app.status = Some((e.to_string(), true));
                }
            }
        }
        KeyCode::Char('s') => {
            if let Err(e) = app.settings_save() {
                app.status = Some((e.to_string(), true));
            }
        }
        _ => {}
    }
    Ok(())
}

fn settings_adjust(app: &mut App, delta: i64) {
    use chrono::Weekday;
    const WEEKDAYS: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    let theme_preview: Option<String> = {
        let AppMode::Settings {
            cursor,
            theme_names,
            theme_idx,
            date_fmt_idx,
            week_start,
            obs_enabled,
            ..
        } = &mut app.mode
        else {
            return;
        };
        match *cursor {
            0 => {
                let n = theme_names.len();
                if n > 0 {
                    *theme_idx = ((*theme_idx as i64 + delta).rem_euclid(n as i64)) as usize;
                }
                theme_names.get(*theme_idx).cloned()
            }
            1 => {
                let n = crate::app::DATE_FORMAT_PRESETS.len();
                *date_fmt_idx = ((*date_fmt_idx as i64 + delta).rem_euclid(n as i64)) as usize;
                None
            }
            2 => {
                let idx = WEEKDAYS.iter().position(|&d| d == *week_start).unwrap_or(0);
                *week_start = WEEKDAYS[((idx as i64 + delta).rem_euclid(7)) as usize];
                None
            }
            3 => {
                *obs_enabled = !*obs_enabled;
                None
            }
            _ => None,
        }
    };

    if let Some(name) = theme_preview {
        let themes = app.themes.clone();
        app.theme = crate::theme::Theme::resolve(&name, &themes);
    }
}
