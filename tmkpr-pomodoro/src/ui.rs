use crate::app::{App, TimerState};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
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

    // Project and Task selection
    let selection_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_projects(f, app, selection_chunks[0]);
    draw_tasks(f, app, selection_chunks[1]);

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

    let paragraph = Paragraph::new(timer_text)
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

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = ["↑↓: Select project  |  ←→: Select task  |  Enter: Start timer",
        "Space: Pause/Resume  |  L: Log session  |  R: Reset  |  Q: Quit"];

    let help = Paragraph::new(help_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}
