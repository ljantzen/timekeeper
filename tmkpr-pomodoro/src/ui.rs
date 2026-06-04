use crate::app::{App, CompletedSession, EditMode, Screen, SoundField, TimerState};

fn hex_to_rgb(hex: &str) -> Option<Color> {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    if app.theme().bg != Color::Reset {
        f.render_widget(
            Block::default().style(Style::default().bg(app.theme().bg)),
            area,
        );
    }

    if app.screen() == Screen::Settings {
        draw_settings(f, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(8),
        ])
        .split(f.area());

    // Timer display
    draw_timer(f, app, chunks[0]);

    // Project, Task, and completed session list
    let selection_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    draw_projects(f, app, selection_chunks[0]);
    draw_tasks(f, app, selection_chunks[1]);
    draw_sessions(f, app, selection_chunks[2]);

    // Status bar
    draw_status(f, app, chunks[2]);

    // Help text
    draw_help(f, app, chunks[3]);

    // Delete confirmation dialog
    if app.edit_mode() == EditMode::ConfirmDelete {
        draw_delete_confirmation(f, app);
    }
}

fn draw_timer(f: &mut Frame, app: &App, area: Rect) {
    let elapsed = app.elapsed();
    let work_duration = app.work_duration();

    let minutes = elapsed.as_secs() / 60;
    let seconds = elapsed.as_secs() % 60;
    let total_min = work_duration / 60;

    let state_label = match app.timer_state() {
        TimerState::Stopped => "STOPPED",
        TimerState::Running => "RUNNING",
        TimerState::Paused => "PAUSED",
        TimerState::Break => "BREAK",
    };

    let timer_text = format!(
        "{:02}:{:02} / {:02}:00 [{}]",
        minutes, seconds, total_min, state_label
    );

    let cycle_info = if app.timer_state() == TimerState::Break {
        let sessions = app.sessions_completed();
        let cycle_size = app.sessions_before_long();
        let current_in_cycle = (sessions % cycle_size) + 1;
        format!("Session {}/{}", current_in_cycle, cycle_size)
    } else if app.sessions_completed() > 0 {
        let sessions = app.sessions_completed();
        let cycle_size = app.sessions_before_long();
        let current_in_cycle = (sessions % cycle_size) + 1;
        format!("Work session {} of {}", current_in_cycle, cycle_size)
    } else {
        let cycle_size = app.sessions_before_long();
        format!("Work session 1 of {}", cycle_size)
    };

    let active_line = if app.timer_state() != TimerState::Stopped {
        match (app.selected_project(), app.selected_task()) {
            (Some(proj), Some(task)) => {
                let color = proj
                    .color
                    .as_deref()
                    .and_then(hex_to_rgb)
                    .unwrap_or(Color::White);
                Line::from(vec![
                    Span::styled(
                        proj.name.clone(),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" / "),
                    Span::styled(task.name.clone(), Style::default().fg(color)),
                ])
            }
            _ => Line::from(""),
        }
    } else {
        Line::from("")
    };

    let paragraph = Paragraph::new(vec![
        Line::from(timer_text),
        Line::from(cycle_info),
        active_line,
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme().border))
            .title("Timer"),
    )
    .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_projects(f: &mut Frame, app: &App, area: Rect) {
    let projects = app.projects();
    let selected = app.selected_project_idx();

    let mut items: Vec<ListItem> = projects
        .iter()
        .enumerate()
        .map(|(idx, proj)| {
            let is_selected = idx == selected;
            let prefix = if is_selected { "▶ " } else { "  " };
            let mut style = Style::default();

            let display_name = if is_selected && app.edit_mode() == EditMode::EditProject {
                format!("{prefix}{}█", app.edit_buf())
            } else {
                format!("{prefix}{}", proj.name)
            };

            if let Some(c) = proj.color.as_deref().and_then(hex_to_rgb) {
                style = style.fg(c);
            }
            if is_selected {
                style = style.add_modifier(Modifier::BOLD);
            }
            ListItem::new(Line::from(Span::styled(display_name, style)))
        })
        .collect();

    if app.is_new_project_editing() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  + {}█", app.new_project_buf()),
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        ))));
    }

    let title = if app.is_new_project_editing() || app.edit_mode() == EditMode::EditProject {
        "Projects (Enter: save  Esc: cancel)"
    } else {
        "Projects"
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().border))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_tasks(f: &mut Frame, app: &App, area: Rect) {
    let all_tasks = app.tasks();
    let filtered_tasks = app.filtered_tasks();
    let selected = app.selected_task_idx();
    let proj_color = app
        .selected_project()
        .and_then(|p| p.color.as_deref())
        .and_then(hex_to_rgb)
        .unwrap_or(Color::White);

    let mut items: Vec<ListItem> = filtered_tasks
        .iter()
        .map(|task| {
            let is_selected = all_tasks.iter().position(|t| t.id == task.id) == Some(selected);
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if task.completed {
                Style::default().fg(app.theme().dim)
            } else {
                let mut s = Style::default().fg(proj_color);
                if is_selected {
                    s = s.add_modifier(Modifier::BOLD);
                }
                s
            };
            let label = if is_selected && app.edit_mode() == EditMode::EditTask {
                format!("{}{}█", prefix, app.edit_buf())
            } else if task.completed {
                format!("{prefix}{} ✓", task.name)
            } else {
                format!("{prefix}{}", task.name)
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    if app.is_new_task_editing() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  + {}█", app.new_task_buf()),
            Style::default()
                .fg(app.theme().accent)
                .add_modifier(Modifier::BOLD),
        ))));
    }

    let title = if app.is_new_task_editing() || app.edit_mode() == EditMode::EditTask {
        "Tasks (Enter: save  Esc: cancel)"
    } else if app.hide_completed_tasks() {
        "Tasks (completed hidden)"
    } else {
        "Tasks"
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().border))
                .title(title),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_sessions(f: &mut Frame, app: &App, area: Rect) {
    let sessions: &[CompletedSession] = app.completed_sessions();
    let total = sessions.len();

    let items: Vec<ListItem> = sessions
        .iter()
        .rev()
        .enumerate()
        .map(|(idx, s)| {
            let num = total - idx;
            let mins = s.duration.as_secs() / 60;
            let secs = s.duration.as_secs() % 60;
            let proj_color = s
                .color
                .as_deref()
                .and_then(hex_to_rgb)
                .unwrap_or(Color::White);
            let line = if s.project.is_empty() {
                Line::from(format!("#{num}  {mins:02}:{secs:02}"))
            } else {
                Line::from(vec![
                    Span::raw(format!("#{num}  ")),
                    Span::styled(&*s.project, Style::default().fg(proj_color)),
                    Span::raw(format!(" / {} ({mins:02}:{secs:02})", s.task)),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().border))
                .title("Completed"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if let Some(msg) = app.message() {
        msg.to_string()
    } else {
        match app.timer_state() {
            TimerState::Stopped => "Ready. Press Enter to start.".to_string(),
            TimerState::Running => "Timer running...".to_string(),
            TimerState::Paused => "Timer paused. Press Space to resume.".to_string(),
            TimerState::Break => "Break time! (Press Space to resume or R to reset)".to_string(),
        }
    };

    let paragraph = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().border))
                .title("Status"),
        )
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let (cfg, cursor) = app.settings_state();
    let editing = app.sound_editing();
    let buf = app.sound_edit_buf();

    let accent = app.theme().accent;
    let sel = move |text: String, selected: bool| -> Line {
        if selected {
            Line::from(Span::styled(
                text,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(text)
        }
    };

    let sound_line = |label: &str, path: Option<&str>, field: SoundField| -> Line {
        let is_editing = editing == Some(field);
        let is_selected = cursor
            == if field == SoundField::WorkToBreak {
                8
            } else {
                9
            };
        let value = if is_editing {
            format!("{buf}█")
        } else {
            path.unwrap_or("not set").to_string()
        };
        let text = format!("  {label:<22} {value}");
        if is_editing {
            Line::from(Span::styled(
                text,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ))
        } else {
            sel(text, is_selected)
        }
    };

    let hint = if editing.is_some() {
        "  Type path   Backspace: delete   Enter: confirm   Esc: discard"
    } else {
        "  ↑↓ navigate   ←→ adjust   Enter: edit/save   Esc: cancel"
    };

    let lines: Vec<Line> = vec![
        Line::from(""),
        sel(
            format!(
                "  Work duration:       ◀  {} min  ▶",
                cfg.work_duration_minutes
            ),
            cursor == 0,
        ),
        sel(
            format!(
                "  Break duration:      ◀  {} min  ▶",
                cfg.break_duration_minutes
            ),
            cursor == 1,
        ),
        sel(
            format!(
                "  Sessions / cycle:    ◀  {}      ▶",
                cfg.sessions_before_long_break
            ),
            cursor == 2,
        ),
        sel(
            format!(
                "  Long break:          ◀  {} min  ▶",
                cfg.long_break_duration_minutes
            ),
            cursor == 3,
        ),
        sel(
            format!(
                "  Max cycles:          ◀  {}  ▶",
                if cfg.max_cycles == 0 {
                    "0 (unlimited)".to_string()
                } else {
                    cfg.max_cycles.to_string()
                }
            ),
            cursor == 4,
        ),
        sel(
            format!(
                "  Desktop notify:         {}",
                if cfg.notify_desktop {
                    "[✓] On"
                } else {
                    "[ ] Off"
                }
            ),
            cursor == 5,
        ),
        sel(
            format!(
                "  Message timeout:     ◀  {} sec  ▶",
                cfg.message_timeout_secs
            ),
            cursor == 6,
        ),
        sel(
            format!(
                "  Auto-start break:       {}",
                if cfg.auto_start_break {
                    "[✓] On"
                } else {
                    "[ ] Off"
                }
            ),
            cursor == 7,
        ),
        Line::from(""),
        sound_line(
            "Sound (work→break):",
            cfg.sound_work_to_break.as_deref(),
            SoundField::WorkToBreak,
        ),
        sound_line(
            "Sound (break→work):",
            cfg.sound_break_to_work.as_deref(),
            SoundField::BreakToWork,
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Formats: WAV · MP3 · OGG · FLAC",
            Style::default().fg(app.theme().dim),
        )),
        Line::from(Span::styled(
            "  Suggested: ~/.config/tmkpr/sounds/",
            Style::default().fg(app.theme().dim),
        )),
        Line::from(""),
        Line::from(hint),
    ];

    let block = Block::default()
        .title("Settings")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme().border));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = [
        "↑↓: Project  |  ←→: Task  |  Enter: Work  |  B: Break  |  Space: Pause  |  C: Complete",
        "N: New task  |  P: New project  |  E: Edit  |  D: Delete  |  H: Hide completed  |  L: Log  |  R: Reset  |  S: Settings  |  Q: Quit",
    ];

    let help = Paragraph::new(help_text.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().border))
                .title("Help"),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}

fn draw_delete_confirmation(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = 40.min(area.width.saturating_sub(4));
    let height = 7;
    let left = (area.width.saturating_sub(width)) / 2;
    let top = area.height.saturating_sub(height) / 2;

    let dialog_area = Rect {
        x: left,
        y: top,
        width,
        height,
    };

    let title = match app.delete_target() {
        Some("project") => "Delete Project?",
        Some("task") => "Delete Task?",
        _ => "Delete?",
    };

    let dialog = Paragraph::new("Press Y to confirm, any other key to cancel")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme().accent))
                .title(title),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        dialog_area,
    );
    f.render_widget(dialog, dialog_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgb_with_hash_prefix() {
        assert_eq!(hex_to_rgb("#ff5733"), Some(Color::Rgb(255, 87, 51)));
    }

    #[test]
    fn hex_to_rgb_without_hash_prefix() {
        assert_eq!(hex_to_rgb("ff5733"), Some(Color::Rgb(255, 87, 51)));
    }

    #[test]
    fn hex_to_rgb_black_and_white() {
        assert_eq!(hex_to_rgb("#000000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(hex_to_rgb("#ffffff"), Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn hex_to_rgb_uppercase() {
        assert_eq!(hex_to_rgb("#FF5733"), Some(Color::Rgb(255, 87, 51)));
    }

    #[test]
    fn hex_to_rgb_wrong_length_returns_none() {
        assert_eq!(hex_to_rgb("#fff"), None);
        assert_eq!(hex_to_rgb("#fffffff"), None);
        assert_eq!(hex_to_rgb(""), None);
        assert_eq!(hex_to_rgb("#"), None);
    }

    #[test]
    fn hex_to_rgb_invalid_chars_returns_none() {
        assert_eq!(hex_to_rgb("#gggggg"), None);
        assert_eq!(hex_to_rgb("#xyz123"), None);
    }
}
