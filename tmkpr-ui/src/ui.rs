use chrono::{Datelike, Local};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, AppMode, ModeKind};
use crate::form::Form;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_active(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);

    match app.mode.kind() {
        ModeKind::StartModal => render_start_modal(frame, app, area),
        ModeKind::EditModal => render_edit_modal(frame, app, area),
        ModeKind::ConfirmDelete => render_confirm_delete(frame, app, area),
        ModeKind::AddProject => render_add_project(frame, app, area),
        ModeKind::AddTask => render_add_task(frame, app, area),
        ModeKind::Filter => render_filter(frame, app, area),
        ModeKind::Comments => render_comments(frame, app, area),
        ModeKind::AddComment => render_add_comment(frame, app, area),
        ModeKind::Help => render_help(frame, area),
        ModeKind::Normal => {}
    }
}

fn render_active(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Active Entry ")
        .borders(Borders::ALL);

    let line = match &app.active_entry {
        Some(entry) => {
            let secs = entry.elapsed().num_seconds();
            let elapsed = format!(
                "{:02}:{:02}:{:02}",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            );
            let what = match (&entry.project_id, &entry.task_id) {
                (Some(pid), Some(tid)) => {
                    format!("{} / {}", app.project_name(pid), app.task_name(tid))
                }
                (Some(pid), None) => app.project_name(pid).to_string(),
                _ => "(no project)".to_string(),
            };
            let note_part = entry
                .note
                .as_deref()
                .filter(|n| !n.is_empty())
                .map(|n| format!("  {n}"))
                .unwrap_or_default();
            Line::from(vec![
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(
                    what,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(note_part),
                Span::raw("  "),
                Span::styled(elapsed, Style::default().fg(Color::Green)),
            ])
        }
        None => Line::from(Span::styled(
            "No active entry",
            Style::default().fg(Color::DarkGray),
        )),
    };

    frame.render_widget(Paragraph::new(line).block(block), area);
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    render_entries(frame, app, chunks[0]);
    render_week(frame, app, chunks[1]);
}

fn render_entries(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.has_filter() {
        let mut parts = Vec::new();
        if !app.filter_project_name.is_empty() {
            parts.push(format!("project: {}", app.filter_project_name));
        }
        if !app.filter_date_str.is_empty() {
            parts.push(app.filter_date_str.clone());
        }
        format!(" Entries ({}) [{}] ", app.entries.len(), parts.join(", "))
    } else {
        format!(" Entries ({}) ", app.entries.len())
    };
    let block = Block::default().title(title).borders(Borders::ALL);

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| {
            let start = entry
                .started_at
                .with_timezone(&Local)
                .format("%H:%M")
                .to_string();
            let end = entry
                .finished_at
                .map(|t| t.with_timezone(&Local).format("%H:%M").to_string())
                .unwrap_or_else(|| "     ".to_string());

            let secs = entry.elapsed().num_seconds();
            let dur = if secs >= 3600 {
                format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
            } else {
                format!("{}m", secs / 60)
            };

            let comment_prefix = if app.entry_has_comments(&entry.id) {
                "[c] "
            } else {
                "    "
            };

            let proj_task = match (&entry.project_id, &entry.task_id) {
                (Some(pid), Some(tid)) => {
                    format!("{}{}/{}", comment_prefix, app.project_name(pid), app.task_name(tid))
                }
                (Some(pid), None) => format!("{}{}", comment_prefix, app.project_name(pid)),
                _ => String::new(),
            };

            let note = entry
                .note
                .as_deref()
                .filter(|n| !n.is_empty())
                .map(|n| format!("  {n}"))
                .unwrap_or_default();

            let tags = if entry.tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", entry.tags.join(", "))
            };

            ListItem::new(format!("{start}-{end}  {dur:<8}{proj_task}{note}{tags}"))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_week(frame: &mut Frame, app: &App, area: Rect) {
    let now = Local::now();
    let week_num = now.iso_week().week();
    let title = format!(" Week W{week_num:02} ");
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = match &app.week_report {
        None => vec![Line::from(Span::styled(
            "No data",
            Style::default().fg(Color::DarkGray),
        ))],
        Some(report) => {
            let mut lines: Vec<Line> = report
                .days
                .iter()
                .map(|day| {
                    let name = day.date.format("%a");
                    let date = day.date.format("%d");
                    let h = day.total_secs / 3600;
                    let m = (day.total_secs % 3600) / 60;
                    let time_str = format!("{h}h {m:02}m");
                    let style = if day.total_secs > 0 {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    Line::styled(format!("{name} {date}  {time_str}"), style)
                })
                .collect();

            lines.push(Line::from("─".repeat(inner.width as usize)));

            let total_h = report.total_secs / 3600;
            let total_m = (report.total_secs % 3600) / 60;
            lines.push(Line::from(Span::styled(
                format!("Total  {total_h}h {total_m:02}m"),
                Style::default().add_modifier(Modifier::BOLD),
            )));

            lines
        }
    };

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let line = match &app.status {
        Some((msg, is_error)) => {
            let style = if *is_error {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Cyan)
            };
            Line::from(Span::styled(msg.clone(), style))
        }
        None => Line::from(Span::styled(
            " [s]tart  [x]stop  [e]dit  [d]el  [f]ilter  [c]omments  [C]comment  [p]roject  [t]ask  [r]efresh  [?]help  [q]uit",
            Style::default().fg(Color::DarkGray),
        )),
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_form_modal(frame: &mut Frame, area: Rect, title: &str, percent_y: u16, form: &Form) {
    let popup_area = centered_rect(65, percent_y, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let n = form.fields.len();
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Length(3))
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();

    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // First pass: render all fields.
    for (i, field) in form.fields.iter().enumerate() {
        let focused = i == form.focused;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let field_block = Block::default()
            .title(Span::styled(field.label, border_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        let display = if focused {
            let before = &field.value[..field.cursor];
            let after = &field.value[field.cursor..];
            format!("{before}█{after}")
        } else {
            field.value.clone()
        };

        frame.render_widget(Paragraph::new(display).block(field_block), field_chunks[i]);
    }

    // Second pass: render autocomplete dropdown on top of all fields so it
    // isn't covered by subsequent field widgets.
    let focused_field = &form.fields[form.focused];
    let suggestions = focused_field.suggestions();
    if !suggestions.is_empty() {
        let max_show: u16 = 6;
        let shown = (suggestions.len() as u16).min(max_show);
        let anchor = field_chunks[form.focused];
        let dropdown = Rect {
            x: anchor.x,
            y: anchor.bottom(),
            width: anchor.width,
            height: shown + 2,
        };
        frame.render_widget(Clear, dropdown);
        let items: Vec<ListItem> = suggestions.iter().map(|s| ListItem::new(*s)).collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut ac_state = ListState::default();
        ac_state.select(focused_field.ac_index);
        frame.render_stateful_widget(list, dropdown, &mut ac_state);
    }

    if let Some(&hint_area) = field_chunks.get(n) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "↓/↑ autocomplete  Tab next field  Enter select/submit  Esc cancel",
                Style::default().fg(Color::DarkGray),
            )),
            hint_area,
        );
    }
}

fn render_start_modal(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::StartModal(form) = &app.mode {
        render_form_modal(frame, area, " Start Entry ", 65, form);
    }
}

fn render_edit_modal(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::EditModal { form, .. } = &app.mode {
        render_form_modal(frame, area, " Edit Entry ", 85, form);
    }
}

fn render_add_project(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddProject(form) = &app.mode {
        render_form_modal(frame, area, " Add Project ", 55, form);
    }
}

fn render_add_task(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddTask(form) = &app.mode {
        render_form_modal(frame, area, " Add Task ", 55, form);
    }
}

fn render_confirm_delete(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ConfirmDelete { display, .. } = &app.mode {
        let popup_area = centered_rect(50, 25, area);
        frame.render_widget(Clear, popup_area);
        let block = Block::default()
            .title(" Confirm Delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Delete {}?", display)),
                Line::from(""),
                Line::from(Span::styled(
                    "[y] Yes  [n/Esc] No",
                    Style::default().fg(Color::DarkGray),
                )),
            ]),
            inner,
        );
    }
}

fn render_help(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(55, 80, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default().title(" Help ").borders(Borders::ALL);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("Navigation", bold)),
            Line::from(vec![
                Span::styled("  j / ↓  ", bold),
                Span::raw("Next entry"),
            ]),
            Line::from(vec![
                Span::styled("  k / ↑  ", bold),
                Span::raw("Previous entry"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Actions", bold)),
            Line::from(vec![
                Span::styled("  s      ", bold),
                Span::raw("Start new entry"),
            ]),
            Line::from(vec![
                Span::styled("  x      ", bold),
                Span::raw("Stop active entry"),
            ]),
            Line::from(vec![
                Span::styled("  e      ", bold),
                Span::raw("Edit selected entry"),
            ]),
            Line::from(vec![
                Span::styled("  d      ", bold),
                Span::raw("Delete selected entry"),
            ]),
            Line::from(vec![
                Span::styled("  f      ", bold),
                Span::raw("Filter entries by project / date"),
            ]),
            Line::from(vec![
                Span::styled("  c      ", bold),
                Span::raw("View/add comments on selected entry"),
            ]),
            Line::from(vec![
                Span::styled("  C      ", bold),
                Span::raw("Add comment to active entry (fails if none running)"),
            ]),
            Line::from(vec![
                Span::styled("  r      ", bold),
                Span::raw("Refresh data"),
            ]),
            Line::from(vec![
                Span::styled("  p      ", bold),
                Span::raw("Add new project"),
            ]),
            Line::from(vec![
                Span::styled("  t      ", bold),
                Span::raw("Add new task"),
            ]),
            Line::from(""),
            Line::from(Span::styled("General", bold)),
            Line::from(vec![
                Span::styled("  ?      ", bold),
                Span::raw("This help screen"),
            ]),
            Line::from(vec![Span::styled("  q/Esc  ", bold), Span::raw("Quit")]),
            Line::from(""),
            Line::from(Span::styled("Any key to close", dim)),
        ]),
        inner,
    );
}

fn render_comments(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::Comments {
        entry_id,
        comments,
        selected,
    } = &app.mode
    else {
        return;
    };

    let display = app.entry_display(entry_id);
    let title = format!(" Comments: {display} ({}) ", comments.len());
    let popup_area = centered_rect(72, 65, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if comments.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "No comments. Press [a] to add one.",
                Style::default().fg(Color::DarkGray),
            )),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = comments
            .iter()
            .map(|c| {
                let ts = c
                    .created_at
                    .with_timezone(&Local)
                    .format("%m-%d %H:%M")
                    .to_string();
                ListItem::new(format!("{ts}  {}", c.body))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(*selected));
        frame.render_stateful_widget(list, chunks[0], &mut state);
    }

    frame.render_widget(
        Paragraph::new(Span::styled(
            "[a] add  [d] delete  [j/k] navigate  [Esc] close",
            Style::default().fg(Color::DarkGray),
        )),
        chunks[1],
    );
}

fn render_filter(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::Filter(form) = &app.mode {
        render_form_modal(frame, area, " Filter Entries ", 65, form);
    }
}

fn render_add_comment(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddComment { entry_id, form } = &app.mode {
        let display = app.entry_display(entry_id);
        let title = format!(" Add Comment: {display} ");
        render_form_modal(frame, area, &title, 35, form);
    }
}
