use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, TaskStatus};

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs > 0 {
        format!("{}.{}s", secs, d.subsec_millis() / 100)
    } else {
        format!("{}ms", d.as_millis())
    }
}

pub fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let workspace_name = app.workspace.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| app.workspace.display().to_string());

    let mut spans = vec![
        Span::styled(
            " MERC ",
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(workspace_name, Style::default().fg(Color::DarkGray)),
    ];

    // Task summary
    if !app.tasks.is_empty() {
        let done = app.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let total = app.tasks.len();
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("tasks {done}/{total}"),
            if done == total { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Yellow) },
        ));
    }

    spans.push(Span::raw("  "));

    // Status / progress / duration
    if let Some(progress) = &app.agent_progress {
        // Show elapsed time while working
        let elapsed = app.request_started
            .map(|s| format_duration(s.elapsed()))
            .unwrap_or_default();
        spans.push(Span::styled(
            format!("⚙ {}/{} {} ({})", progress.round, progress.max_rounds, progress.current_action, elapsed),
            Style::default().fg(Color::Yellow),
        ));
    } else if app.loading {
        let elapsed = app.request_started
            .map(|s| format_duration(s.elapsed()))
            .unwrap_or_default();
        let tool_count = app.pending_tools.len();
        if tool_count > 0 {
            spans.push(Span::styled(
                format!("⏳ thinking ({tool_count} calls, {elapsed})"),
                Style::default().fg(Color::Yellow),
            ));
        } else {
            spans.push(Span::styled(
                format!("⏳ thinking ({elapsed})"),
                Style::default().fg(Color::Yellow),
            ));
        }
    } else if let Some(d) = &app.last_duration {
        spans.push(Span::styled(
            format!("cooked in {}", format_duration(*d)),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        spans.push(Span::styled("ready", Style::default().fg(Color::Green)));
    }

    let status = Line::from(spans);
    let paragraph = Paragraph::new(status);
    f.render_widget(paragraph, area);
}
