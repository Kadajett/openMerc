use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, SideTab, TaskStatus};

/// Draw the tabbed side panel: Diff | Log | Tasks
pub fn draw_diff_panel(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 12 || area.height < 6 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
        ])
        .split(area);

    // Tab bar
    let tab_titles = vec![
        Span::styled(" Diff ", if app.side_tab == SideTab::Diff {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::DarkGray) }),
        Span::raw(" "),
        Span::styled(" Honcho ", if app.side_tab == SideTab::Log {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::DarkGray) }),
        Span::raw(" "),
        Span::styled(" Tasks ", if app.side_tab == SideTab::Tasks {
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::DarkGray) }),
    ];
    let tab_line = Paragraph::new(Line::from(tab_titles))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(tab_line, chunks[0]);

    match app.side_tab {
        SideTab::Diff => draw_diff_tab(f, app, chunks[1]),
        SideTab::Log => draw_log_tab(f, app, chunks[1]),
        SideTab::Tasks => draw_tasks_tab(f, app, chunks[1]),
    }
}

fn draw_diff_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.modified_files.is_empty() {
        let msg = Paragraph::new("No files modified yet")
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, area);
        return;
    }

    let file_list_height = (app.modified_files.len() as u16 + 2).min(area.height / 3);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(file_list_height), Constraint::Min(3)])
        .split(area);

    let items: Vec<ListItem> = app.modified_files.iter().enumerate().map(|(i, fd)| {
        let style = if i == app.diff_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::Yellow) };
        let prefix = if i == app.diff_selected { ">" } else { " " };
        ListItem::new(Line::from(Span::styled(format!("{prefix} {}", fd.path), style)))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
    f.render_widget(list, chunks[0]);

    if let Some(fd) = app.modified_files.get(app.diff_selected) {
        let mut lines: Vec<Line> = Vec::new();
        for line_text in fd.diff.lines() {
            let style = if line_text.starts_with('+') && !line_text.starts_with("+++") {
                Style::default().fg(Color::Green)
            } else if line_text.starts_with('-') && !line_text.starts_with("---") {
                Style::default().fg(Color::Red)
            } else if line_text.starts_with("@@") {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if line_text.starts_with("---") || line_text.starts_with("+++") {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            lines.push(Line::from(Span::styled(line_text.to_string(), style)));
        }
        let visible = chunks[1].height.saturating_sub(2) as usize;
        let total = lines.len();
        let scroll = if app.diff_scroll == 0 { total.saturating_sub(visible) as u16 } else {
            (total.saturating_sub(visible) as u16).saturating_sub(app.diff_scroll)
        };
        let diff_view = Paragraph::new(lines)
            .block(Block::default().title(format!(" {} ", fd.path)).borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
            .scroll((scroll, 0));
        f.render_widget(diff_view, chunks[1]);
    }
}

fn draw_log_tab(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    if app.change_log.is_empty() {
        lines.push(Line::from(Span::styled("Waiting for Honcho summaries...", Style::default().fg(Color::DarkGray))));
    } else {
        for entry in &app.change_log {
            let time = entry.timestamp.format("%H:%M:%S").to_string();
            lines.push(Line::from(vec![
                Span::styled(time, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(entry.summary.clone(), Style::default().fg(Color::White)),
            ]));
        }
    }
    let visible = area.height.saturating_sub(2) as usize;
    let total = lines.len();
    let scroll = if app.log_scroll == 0 { total.saturating_sub(visible) as u16 } else {
        (total.saturating_sub(visible) as u16).saturating_sub(app.log_scroll)
    };
    let log_view = Paragraph::new(lines)
        .block(Block::default().title(" Honcho Summaries ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)))
        .scroll((scroll, 0));
    f.render_widget(log_view, area);
}

fn draw_tasks_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.tasks.is_empty() {
        let msg = Paragraph::new("No tasks")
            .block(Block::default().title(" Tasks ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green)))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, area);
        return;
    }
    let done = app.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let total = app.tasks.len();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {done}/{total} completed"),
        Style::default().fg(if done == total { Color::Green } else { Color::Yellow }),
    )));
    lines.push(Line::from(""));
    for task in &app.tasks {
        let (icon, color) = match task.status {
            TaskStatus::Completed => ("✓", Color::Green),
            TaskStatus::InProgress => ("→", Color::Yellow),
            TaskStatus::Blocked => ("✗", Color::Red),
            TaskStatus::Pending => ("○", Color::DarkGray),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {icon} "), Style::default().fg(color)),
            Span::styled(format!("[P{}] ", task.priority), Style::default().fg(Color::DarkGray)),
            Span::styled(task.title.clone(), Style::default().fg(Color::White)),
        ]));
        if let Some(desc) = &task.description {
            let short = crate::logger::safe_truncate(desc, 40);
            lines.push(Line::from(Span::styled(format!("     {short}"), Style::default().fg(Color::DarkGray))));
        }
        if let Some(note) = task.notes.last() {
            let short = crate::logger::safe_truncate(note, 40);
            lines.push(Line::from(Span::styled(format!("     {short}"), Style::default().fg(Color::Rgb(100, 100, 140)))));
        }
    }
    let visible = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();
    let scroll = if app.tasks_scroll == 0 { total_lines.saturating_sub(visible) as u16 } else {
        (total_lines.saturating_sub(visible) as u16).saturating_sub(app.tasks_scroll)
    };
    let tasks_view = Paragraph::new(lines)
        .block(Block::default().title(" Tasks ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green)))
        .scroll((scroll, 0));
    f.render_widget(tasks_view, area);
}
