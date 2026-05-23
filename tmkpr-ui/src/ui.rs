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
use crate::theme::Theme;

// Layout constants
#[allow(dead_code)]
mod layout {
    pub const MODAL_WIDTH: u16 = 60;
    pub const MODAL_HEIGHT: u16 = 75;
    pub const ADD_PROJECT_WIDTH: u16 = 55;
    pub const EDIT_PROJECT_WIDTH: u16 = 55;
    pub const ADD_TASK_WIDTH: u16 = 55;
    pub const EDIT_TASK_WIDTH: u16 = 55;
    pub const FILTER_ENTRIES_WIDTH: u16 = 65;
    pub const FILTER_TASKS_WIDTH: u16 = 65;
    pub const FILTER_PROJECTS_WIDTH: u16 = 65;
    pub const COMMENTS_WIDTH: u16 = 70;
    pub const ADD_COMMENT_WIDTH: u16 = 35;
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn project_color(app: &App, project_id: &str) -> Option<Color> {
    app.projects
        .iter()
        .find(|p| p.id == project_id)
        .and_then(|p| p.color.as_ref())
        .and_then(|c| parse_hex_color(c))
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Fill background and set base fg so light themes render readable text.
    if app.theme.bg != Color::Reset || app.theme.fg.is_some() {
        let mut style = Style::default().bg(app.theme.bg);
        if let Some(fg) = app.theme.fg {
            style = style.fg(fg);
        }
        frame.render_widget(Block::default().style(style), area);
    }

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
        ModeKind::ManageProjects => render_manage_projects(frame, app, area),
        ModeKind::EditProject => render_edit_project(frame, app, area),
        ModeKind::FilterProjects => render_filter_projects(frame, app, area),
        ModeKind::AddTask => render_add_task(frame, app, area),
        ModeKind::ManageTasks => render_manage_tasks(frame, app, area),
        ModeKind::EditTask => render_edit_task(frame, app, area),
        ModeKind::Filter => render_filter(frame, app, area),
        ModeKind::FilterTasks => render_filter_tasks(frame, app, area),
        ModeKind::Comments => render_comments(frame, app, area),
        ModeKind::AddComment => render_add_comment(frame, app, area),
        ModeKind::EditComment => render_edit_comment(frame, app, area),
        ModeKind::ConfirmCreate => render_confirm_create(frame, app, area),
        ModeKind::ConfirmDeleteProject => render_confirm_delete_project(frame, app, area),
        ModeKind::Help => render_help(frame, area, &app.theme),
        ModeKind::Normal | ModeKind::Command => {}
    }
}

fn render_active(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Active Entry ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));

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
                Span::styled("● ", Style::default().fg(app.theme.active)),
                Span::styled(
                    what,
                    Style::default()
                        .fg(app.theme.active)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(note_part),
                Span::raw("  "),
                Span::styled(elapsed, Style::default().fg(app.theme.active)),
            ])
        }
        None => Line::from(Span::styled(
            "No active entry",
            Style::default().fg(app.theme.dim),
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
        if !app.entry_filter.project_name.is_empty() {
            parts.push(format!("project: {}", app.entry_filter.project_name));
        }
        if !app.entry_filter.date_str.is_empty() {
            parts.push(app.entry_filter.date_str.clone());
        }
        let sort_part = if app.entry_sort != crate::app::EntrySort::StartDesc {
            format!(" [{}]", app.entry_sort.label())
        } else {
            String::new()
        };
        format!(
            " Entries ({}) [{}]{} ",
            app.entries.len(),
            parts.join(", "),
            sort_part
        )
    } else if app.entry_sort != crate::app::EntrySort::StartDesc {
        format!(
            " Entries ({}) [{}] ",
            app.entries.len(),
            app.entry_sort.label()
        )
    } else {
        format!(" Entries ({}) ", app.entries.len())
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| {
            let fmt = &app.date_format;
            let start_local = entry.started_at.with_timezone(&Local);
            let start = start_local.format(fmt).to_string();
            let end = match entry.finished_at {
                None => "     ".to_string(),
                Some(t) => {
                    let end_local = t.with_timezone(&Local);
                    if end_local.date_naive() == start_local.date_naive() {
                        end_local.format("%H:%M").to_string()
                    } else {
                        end_local.format(fmt).to_string()
                    }
                }
            };

            let secs = entry.elapsed().num_seconds();
            let dur = if secs >= 3600 {
                format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
            } else {
                format!("{}m", secs / 60)
            };

            let comment_indicator = if app.entry_has_comments(&entry.id) {
                "c "
            } else {
                "  "
            };

            let dim = Style::default().fg(app.theme.dim);
            let mut spans: Vec<Span> = vec![
                Span::styled(format!("{start} – {end}  {dur:<8}"), dim),
                Span::styled(comment_indicator, dim),
            ];

            // Add project and task with colors
            match (&entry.project_id, &entry.task_id) {
                (Some(pid), Some(tid)) => {
                    let proj_name = app.project_name(pid);
                    let task_name = app.task_name(tid);
                    let color = project_color(app, pid);
                    let style = color.map(|c| Style::default().fg(c)).unwrap_or_default();
                    spans.push(Span::styled(format!("{}/{}", proj_name, task_name), style));
                }
                (Some(pid), None) => {
                    let proj_name = app.project_name(pid);
                    let color = project_color(app, pid);
                    let style = color.map(|c| Style::default().fg(c)).unwrap_or_default();
                    spans.push(Span::styled(proj_name.to_string(), style));
                }
                _ => {}
            }

            if let Some(note) = entry.note.as_deref().filter(|n| !n.is_empty()) {
                spans.push(Span::styled(format!("  {note}"), dim));
            }

            if !entry.tags.is_empty() {
                spans.push(Span::styled(format!("  [{}]", entry.tags.join(", ")), dim));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(app.theme.selection))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_week(frame: &mut Frame, app: &App, area: Rect) {
    let now = Local::now();
    let week_num = now.iso_week().week();
    let title = format!(" Week W{week_num:02} ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = match &app.week_report {
        None => vec![Line::from(Span::styled(
            "No data",
            Style::default().fg(app.theme.dim),
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
                        Style::default().fg(app.theme.dim)
                    };
                    Line::styled(format!("{name} {date}  {time_str}"), style)
                })
                .collect();

            lines.push(Line::from(Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(app.theme.border),
            )));

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
    use crate::app::ModeKind;
    if app.mode.kind() == ModeKind::Command {
        if let Some((completions, sel_idx)) = app.command_completion_state() {
            let max_visible: usize = 10;
            let visible = completions.len().min(max_visible);
            let popup_h = (visible + 2) as u16;
            let popup_w = completions
                .iter()
                .map(|s| s.len() + 4)
                .max()
                .unwrap_or(24)
                .max(24) as u16;
            let popup_w = popup_w.min(area.width);
            let popup_y = area.y.saturating_sub(popup_h);
            let popup_area = Rect {
                x: area.x,
                y: popup_y,
                width: popup_w,
                height: popup_h,
            };

            // Scroll window to keep selected item visible
            let start = if let Some(idx) = sel_idx {
                idx.saturating_sub(max_visible / 2)
                    .min(completions.len().saturating_sub(visible))
            } else {
                0
            };
            let end = (start + visible).min(completions.len());

            let items: Vec<ListItem> = completions[start..end]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let abs = start + i;
                    let selected = sel_idx == Some(abs);
                    let style = if selected {
                        Style::default()
                            .fg(app.theme.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(app.theme.fg.unwrap_or(Color::White))
                    };
                    let prefix = if selected { "▶ " } else { "  " };
                    ListItem::new(Line::from(Span::styled(format!("{prefix}{name}"), style)))
                })
                .collect();

            let b = app.command_buf().trim();
            let title = if b == "theme" || b.starts_with("theme ") {
                "Themes"
            } else if b == "set date-format" || b.starts_with("set date-format ") {
                "Date Format"
            } else if b == "set" || b.starts_with("set ") {
                "Settings"
            } else {
                "Commands"
            };
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(Style::default().bg(app.theme.bg)),
            );
            frame.render_widget(Clear, popup_area);
            frame.render_widget(list, popup_area);
        }

        let buf = app.command_buf();
        let line = Line::from(vec![
            Span::styled(":", Style::default().fg(app.theme.accent)),
            Span::styled(buf, Style::default().fg(app.theme.accent)),
            Span::styled("█", Style::default().fg(app.theme.accent)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let line = match &app.status {
        Some((msg, is_error)) => {
            let style = if *is_error {
                Style::default().fg(app.theme.error)
            } else {
                Style::default().fg(app.theme.accent)
            };
            Line::from(Span::styled(msg.clone(), style))
        }
        None => Line::from(Span::styled(
            " [s]tart  [S]tart selected  [x]stop  [e]dit  [d]el  [m]erge  [g]ap-fill  [f]ilter  [o]rder  [T/Y/W]  [c]omments  [p]rojects  [:]command  [?]",
            Style::default().fg(app.theme.dim),
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

fn render_form_modal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    percent_y: u16,
    form: &Form,
    theme: &Theme,
) {
    let popup_area = centered_rect(layout::FILTER_ENTRIES_WIDTH, percent_y, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));
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
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.dim)
        };
        let field_block = Block::default()
            .title(Span::styled(field.label, border_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        let text = if focused {
            let before = &field.value[..field.cursor];
            let after = &field.value[field.cursor..];
            format!("{before}█{after}")
        } else {
            field.value.clone()
        };

        let value_color = field
            .completions
            .iter()
            .position(|c| c == &field.value)
            .and_then(|i| field.completion_colors.get(i))
            .and_then(|c| c.as_deref())
            .and_then(parse_hex_color);

        let display = match value_color {
            Some(c) => Line::from(Span::styled(text, Style::default().fg(c))),
            None => Line::from(text),
        };

        frame.render_widget(Paragraph::new(display).block(field_block), field_chunks[i]);
    }

    // Second pass: render autocomplete dropdown on top of all fields so it
    // isn't covered by subsequent field widgets.
    let focused_field = &form.fields[form.focused];
    let suggestions = focused_field.suggestions_colored();
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
        let items: Vec<ListItem> = suggestions
            .iter()
            .map(|(name, color)| {
                let style = color
                    .and_then(parse_hex_color)
                    .map(|c| Style::default().fg(c))
                    .unwrap_or_default();
                ListItem::new(Line::from(Span::styled(*name, style)))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent))
                    .style(Style::default().bg(theme.bg)),
            )
            .highlight_style(Style::default().bg(theme.selection));
        let mut ac_state = ListState::default();
        ac_state.select(focused_field.ac_index);
        frame.render_stateful_widget(list, dropdown, &mut ac_state);
    }

    if let Some(&hint_area) = field_chunks.get(n) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "↓/↑ autocomplete  Tab next field  Enter select/submit  Esc cancel",
                Style::default().fg(theme.dim),
            )),
            hint_area,
        );
    }
}

fn render_start_modal(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::StartModal(form) = &app.mode {
        render_form_modal(frame, area, " Start Entry ", 65, form, &app.theme);
    }
}

fn render_edit_modal(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::EditModal { form, .. } = &app.mode {
        render_form_modal(frame, area, " Edit Entry ", 85, form, &app.theme);
    }
}

fn render_add_project(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddProject(form) = &app.mode {
        render_form_modal(
            frame,
            area,
            " Add Project ",
            layout::ADD_PROJECT_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_list_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: Vec<ListItem>,
    selected: usize,
    empty_msg: &str,
    theme: &Theme,
) {
    let popup_area = centered_rect(layout::MODAL_WIDTH, layout::MODAL_HEIGHT, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(empty_msg, Style::default().fg(theme.dim))),
            chunks[0],
        );
    } else {
        let list = List::new(items)
            .highlight_style(Style::default().bg(theme.selection))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(selected));
        frame.render_stateful_widget(list, chunks[0], &mut state);
    }

    frame.render_widget(
        Paragraph::new(Span::styled(
            "[a] add  [e] edit  [d] delete  [s] sort  [f] filter  [j/k] navigate  [Esc] close",
            Style::default().fg(theme.dim),
        )),
        chunks[1],
    );
}

fn render_manage_projects(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ManageProjects { projects, selected } = &app.mode {
        let sort_info = app.project_sort.label();
        let filter_info = if app.project_filter.hide_archived {
            " [active]"
        } else {
            ""
        };
        let title = format!(
            " Projects ({}){} [{}] ",
            projects.len(),
            filter_info,
            sort_info
        );

        let items: Vec<ListItem> = projects
            .iter()
            .map(|p| {
                let color = p.color.as_ref().and_then(|c| parse_hex_color(c));
                let style = color.map(|c| Style::default().fg(c)).unwrap_or_default();
                let color_indicator = color.map(|_| "●").unwrap_or(" ");

                let mut spans = vec![Span::raw(format!("{} ", color_indicator))];
                spans.push(Span::styled(p.name.clone(), style));

                ListItem::new(Line::from(spans))
            })
            .collect();

        render_list_panel(
            frame,
            area,
            &title,
            items,
            *selected,
            "No projects. Press [a] to add one.",
            &app.theme,
        );
    }
}

fn render_edit_project(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::EditProject { form, .. } = &app.mode {
        render_form_modal(
            frame,
            area,
            " Edit Project ",
            layout::EDIT_PROJECT_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_add_task(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddTask(form) = &app.mode {
        render_form_modal(
            frame,
            area,
            " Add Task ",
            layout::ADD_TASK_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_manage_tasks(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ManageTasks { tasks, selected } = &app.mode {
        let sort_info = app.task_sort.label();
        let filter_info = if app.task_filter.project_id.is_some() || app.task_filter.hide_archived {
            let mut info = String::new();
            if let Some(pid) = &app.task_filter.project_id {
                if let Some(proj) = app.projects.iter().find(|p| &p.id == pid) {
                    info.push_str(&format!("[{}]", proj.name));
                }
            }
            if app.task_filter.hide_archived {
                if !info.is_empty() {
                    info.push(' ');
                }
                info.push_str("[active]");
            }
            format!(" {}", info)
        } else {
            String::new()
        };
        let title = format!(" Tasks ({}){} [{}] ", tasks.len(), filter_info, sort_info);

        let items: Vec<ListItem> = tasks
            .iter()
            .map(|t| {
                let proj_name = app.project_name(&t.project_id);
                if t.completed {
                    let style = Style::default().fg(app.theme.dim);
                    let line = Line::from(vec![
                        Span::styled(format!("{} (", t.name), style),
                        Span::styled(proj_name.to_string(), style),
                        Span::styled(") ✓", style),
                    ]);
                    ListItem::new(line)
                } else {
                    let color = project_color(app, &t.project_id);
                    let style = color.map(|c| Style::default().fg(c)).unwrap_or_default();

                    let mut spans = vec![Span::raw(format!("{} (", t.name))];
                    spans.push(Span::styled(proj_name.to_string(), style));
                    spans.push(Span::raw(")"));
                    ListItem::new(Line::from(spans))
                }
            })
            .collect();

        let popup_area = centered_rect(layout::MODAL_WIDTH, layout::MODAL_HEIGHT, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border))
            .style(Style::default().bg(app.theme.bg));
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        if items.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No tasks. Press [a] to add one.",
                    Style::default().fg(app.theme.dim),
                )),
                chunks[0],
            );
        } else {
            let list = List::new(items)
                .highlight_style(Style::default().bg(app.theme.selection))
                .highlight_symbol("> ");

            let mut state = ListState::default();
            state.select(Some(*selected));
            frame.render_stateful_widget(list, chunks[0], &mut state);
        }

        frame.render_widget(
            Paragraph::new(Span::styled(
                "[a] add  [e] edit  [c] complete  [d] delete  [s] sort  [f] filter  [j/k] navigate  [Esc] close",
                Style::default().fg(app.theme.dim),
            )),
            chunks[1],
        );
    }
}

fn render_edit_task(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::EditTask { form, .. } = &app.mode {
        render_form_modal(
            frame,
            area,
            " Edit Task ",
            layout::EDIT_TASK_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_confirm_delete(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ConfirmDelete { display, .. } = &app.mode {
        let popup_area = centered_rect(50, 25, area);
        frame.render_widget(Clear, popup_area);
        let block = Block::default()
            .title(" Confirm Delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.error))
            .style(Style::default().bg(app.theme.bg));
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Delete {}?", display)),
                Line::from(""),
                Line::from(Span::styled(
                    "[y] Yes  [n/Esc] No",
                    Style::default().fg(app.theme.dim),
                )),
            ]),
            inner,
        );
    }
}

fn render_confirm_delete_project(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ConfirmDeleteProject { name, .. } = &app.mode {
        let popup_area = centered_rect(50, 25, area);
        frame.render_widget(Clear, popup_area);
        let block = Block::default()
            .title(" Delete Project ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.error))
            .style(Style::default().bg(app.theme.bg));
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Permanently delete \"{}\"?", name)),
                Line::from(""),
                Line::from(Span::styled(
                    "[y] Yes  [n/Esc] No",
                    Style::default().fg(app.theme.dim),
                )),
            ]),
            inner,
        );
    }
}

fn render_confirm_create(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::ConfirmCreate {
        project,
        task,
        create_project,
        create_task,
        ..
    } = &app.mode
    {
        let popup_area = centered_rect(60, 25, area);
        frame.render_widget(Clear, popup_area);
        let block = Block::default()
            .title(" Create? ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.warning))
            .style(Style::default().bg(app.theme.bg));
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let msg = match (*create_project, *create_task) {
            (true, true) => format!("Project \"{project}\" and task \"{task}\" not found."),
            (true, false) => format!("Project \"{project}\" not found."),
            (false, true) => format!("Task \"{task}\" not found in project \"{project}\"."),
            (false, false) => String::new(),
        };

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(msg),
                Line::from(""),
                Line::from(Span::styled(
                    "[y] Create and start  [n/Esc] Cancel",
                    Style::default().fg(app.theme.dim),
                )),
            ]),
            inner,
        );
    }
}

fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let popup_area = centered_rect(layout::ADD_PROJECT_WIDTH, 80, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.dim);

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
            Line::from(Span::styled("Entry actions", bold)),
            Line::from(vec![
                Span::styled("  s      ", bold),
                Span::raw("Start new entry"),
            ]),
            Line::from(vec![
                Span::styled("  S      ", bold),
                Span::raw("Start from selected (same project / task / note)"),
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
                Span::styled("  m      ", bold),
                Span::raw("Merge selected into next entry with same project/task"),
            ]),
            Line::from(vec![
                Span::styled("  g      ", bold),
                Span::raw("Extend selected entry's start/end to same-day neighbours"),
            ]),
            Line::from(vec![
                Span::styled("  G      ", bold),
                Span::raw("Extend active entry's start to same-day prior entry"),
            ]),
            Line::from(vec![
                Span::styled("  c      ", bold),
                Span::raw("View / add comments on selected entry"),
            ]),
            Line::from(vec![
                Span::styled("  C      ", bold),
                Span::raw("View / add comments on active entry"),
            ]),
            Line::from(vec![Span::styled("  r      ", bold), Span::raw("Refresh")]),
            Line::from(""),
            Line::from(Span::styled("Filtering & sorting", bold)),
            Line::from(vec![
                Span::styled("  f      ", bold),
                Span::raw("Filter by project / date"),
            ]),
            Line::from(vec![
                Span::styled("  o      ", bold),
                Span::raw("Cycle sort order"),
            ]),
            Line::from(vec![
                Span::styled("  T      ", bold),
                Span::raw("Quick filter: Today"),
            ]),
            Line::from(vec![
                Span::styled("  Y      ", bold),
                Span::raw("Quick filter: Yesterday"),
            ]),
            Line::from(vec![
                Span::styled("  W      ", bold),
                Span::raw("Quick filter: This week"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Projects  [p]", bold)),
            Line::from(vec![
                Span::styled("  a      ", bold),
                Span::raw("Add project"),
            ]),
            Line::from(vec![
                Span::styled("  e      ", bold),
                Span::raw("Edit selected project"),
            ]),
            Line::from(vec![
                Span::styled("  s      ", bold),
                Span::raw("Cycle sort"),
            ]),
            Line::from(vec![Span::styled("  f      ", bold), Span::raw("Filter")]),
            Line::from(""),
            Line::from(Span::styled("Tasks  [t]", bold)),
            Line::from(vec![Span::styled("  a      ", bold), Span::raw("Add task")]),
            Line::from(vec![
                Span::styled("  e      ", bold),
                Span::raw("Edit selected task"),
            ]),
            Line::from(vec![
                Span::styled("  d      ", bold),
                Span::raw("Delete selected task"),
            ]),
            Line::from(vec![
                Span::styled("  s      ", bold),
                Span::raw("Cycle sort"),
            ]),
            Line::from(vec![Span::styled("  f      ", bold), Span::raw("Filter")]),
            Line::from(""),
            Line::from(Span::styled("Comments  [c / C]", bold)),
            Line::from(vec![
                Span::styled("  a      ", bold),
                Span::raw("Add comment"),
            ]),
            Line::from(vec![
                Span::styled("  d      ", bold),
                Span::raw("Delete selected comment"),
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border))
        .style(Style::default().bg(app.theme.bg));
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
                Style::default().fg(app.theme.dim),
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
            .highlight_style(Style::default().bg(app.theme.selection))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(*selected));
        frame.render_stateful_widget(list, chunks[0], &mut state);
    }

    frame.render_widget(
        Paragraph::new(Span::styled(
            "[a] add  [e] edit  [d] delete  [j/k] navigate  [Esc] close",
            Style::default().fg(app.theme.dim),
        )),
        chunks[1],
    );
}

fn render_filter(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::Filter(form) = &app.mode {
        render_form_modal(
            frame,
            area,
            " Filter Entries ",
            layout::FILTER_ENTRIES_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_filter_tasks(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::FilterTasks(form) = &app.mode {
        render_form_modal(
            frame,
            area,
            " Filter Tasks ",
            layout::FILTER_TASKS_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_filter_projects(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::FilterProjects(form) = &app.mode {
        render_form_modal(
            frame,
            area,
            " Filter Projects ",
            layout::FILTER_PROJECTS_WIDTH,
            form,
            &app.theme,
        );
    }
}

fn render_add_comment(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::AddComment { entry_id, form } = &app.mode {
        let display = app.entry_display(entry_id);
        let title = format!(" Add Comment: {display} ");
        render_form_modal(frame, area, &title, 35, form, &app.theme);
    }
}

fn render_edit_comment(frame: &mut Frame, app: &App, area: Rect) {
    if let AppMode::EditComment { entry_id, form, .. } = &app.mode {
        let display = app.entry_display(entry_id);
        let title = format!(" Edit Comment: {display} ");
        render_form_modal(frame, area, &title, 35, form, &app.theme);
    }
}
