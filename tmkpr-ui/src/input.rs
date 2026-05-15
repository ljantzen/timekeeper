use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppMode, ModeKind};
use crate::form::FormResult;

pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.running = false;
        return Ok(());
    }

    match app.mode_kind() {
        ModeKind::Normal => handle_normal(app, key),
        ModeKind::StartModal => handle_start_modal(app, key),
        ModeKind::EditModal => handle_edit_modal(app, key),
        ModeKind::ConfirmDelete => handle_confirm_delete(app, key),
        ModeKind::AddProject => handle_add_project(app, key),
        ModeKind::AddTask => handle_add_task(app, key),
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
        KeyCode::Char('r') => match app.refresh() {
            Ok(()) => app.status = Some(("Refreshed.".into(), false)),
            Err(e) => app.status = Some((e.to_string(), true)),
        },
        KeyCode::Char('p') => app.open_add_project_modal(),
        KeyCode::Char('t') => app.open_add_task_modal(),
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        _ => {}
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
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddProject(form) = old {
                let name = form.fields[0].value.clone();
                let description = form.fields[1].value.clone();
                let color = form.fields[2].value.clone();
                if let Err(e) = app.add_project(&name, &description, &color) {
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
            app.mode = AppMode::Normal;
        }
        FormResult::Submit => {
            let old = std::mem::replace(&mut app.mode, AppMode::Normal);
            if let AppMode::AddTask(form) = old {
                let project = form.fields[0].value.clone();
                let name = form.fields[1].value.clone();
                let description = form.fields[2].value.clone();
                if let Err(e) = app.add_task(&project, &name, &description) {
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
                let project = form.fields[0].value.clone();
                let task = form.fields[1].value.clone();
                let note = form.fields[2].value.clone();
                let tags = form.fields[3].value.clone();
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
                let project = form.fields[0].value.clone();
                let task = form.fields[1].value.clone();
                let note = form.fields[2].value.clone();
                let start = form.fields[3].value.clone();
                let end = form.fields[4].value.clone();
                let tags = form.fields[5].value.clone();
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
