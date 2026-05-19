use crate::app::{App, CompletedSession, Screen, TimerState};
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
    if app.screen() == Screen::Settings {
        draw_settings(f, app, f.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
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
    draw_help(f, chunks[3]);
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
        "Session 1 of X".to_string()
    };

    let full_text = format!("{}\n{}", timer_text, cycle_info);

    let paragraph = Paragraph::new(full_text)
        .block(Block::default().borders(Borders::ALL).title("Timer"))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_projects(f: &mut Frame, app: &App, area: Rect) {
    let projects = app.projects();
    let selected = app.selected_project_idx();

    let items: Vec<ListItem> = projects
        .iter()
        .enumerate()
        .map(|(idx, proj)| {
            let content = if idx == selected {
                format!("▶ {}", proj.name)
            } else {
                format!("  {}", proj.name)
            };
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Projects"))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_tasks(f: &mut Frame, app: &App, area: Rect) {
    let tasks = app.tasks();
    let selected = app.selected_task_idx();

    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(idx, task)| {
            let content = if idx == selected {
                format!("▶ {}", task.name)
            } else {
                format!("  {}", task.name)
            };
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
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
            let text = if s.project.is_empty() {
                format!("#{num}  {mins:02}:{secs:02}")
            } else {
                format!("#{num}  {} / {} ({mins:02}:{secs:02})", s.project, s.task)
            };
            ListItem::new(text)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Completed"))
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
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let settings_lines = vec![
        Line::from(""),
        Line::from(format!(
            "  Work duration:       ◀  {} min  ▶",
            app.settings_state().0.work_duration_minutes
        )),
        Line::from(format!(
            "  Break duration:      ◀  {} min  ▶",
            app.settings_state().0.break_duration_minutes
        )),
        Line::from(format!(
            "  Sessions / cycle:    ◀  {}      ▶",
            app.settings_state().0.sessions_before_long_break
        )),
        Line::from(format!(
            "  Long break:          ◀  {} min  ▶",
            app.settings_state().0.long_break_duration_minutes
        )),
        Line::from(format!(
            "  Bell:                   {}",
            if app.settings_state().0.notify_bell { "[✓] On" } else { "[ ] Off" }
        )),
        Line::from(format!(
            "  Desktop notify:         {}",
            if app.settings_state().0.notify_desktop { "[✓] On" } else { "[ ] Off" }
        )),
        Line::from(format!(
            "  Message timeout:     ◀  {} sec  ▶",
            app.settings_state().0.message_timeout_secs
        )),
        Line::from(format!(
            "  Auto-start break:       {}",
            if app.settings_state().0.auto_start_break { "[✓] On" } else { "[ ] Off" }
        )),
        Line::from(""),
        Line::from("  ↑↓ select   ←→ adjust   Enter save   Esc cancel"),
    ];

    let mut styled_lines = Vec::new();
    for (idx, line) in settings_lines.iter().enumerate() {
        if idx == app.settings_state().1 + 1 && idx > 0 && idx < 10 {
            styled_lines.push(
                Line::from(vec![Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )])
            );
        } else {
            styled_lines.push(line.clone());
        }
    }

    let block = Block::default()
        .title("Settings")
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(styled_lines)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = ["↑↓: Select project  |  ←→: Select task  |  Enter: Work  |  B: Break",
        "Space: Pause/Resume  |  L: Log  |  R: Reset  |  S: Settings  |  Q: Quit"];

    let help = Paragraph::new(help_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}
