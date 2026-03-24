use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Role};
use super::render;

pub fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" openMerc ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_width = area.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.conversation.messages {
        match msg.role {
            Role::User => {
                lines.push(Line::from(vec![
                    Span::styled(" you ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(msg.timestamp.format("%H:%M").to_string(), Style::default().fg(Color::DarkGray)),
                ]));
                let rendered = render::render_message(&msg.content, "  ");
                for line in rendered {
                    let wrapped = wrap_line(line, inner_width);
                    lines.extend(wrapped);
                }
                lines.push(Line::from(""));
            }
            Role::Assistant => {
                lines.push(Line::from(vec![
                    Span::styled(" merc ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(msg.timestamp.format("%H:%M").to_string(), Style::default().fg(Color::DarkGray)),
                ]));
                let rendered = render::render_message(&msg.content, "  ");
                for line in rendered {
                    let wrapped = wrap_line(line, inner_width);
                    lines.extend(wrapped);
                }
                lines.push(Line::from(""));
            }
            Role::System => {
                // System messages in dim, no label
                for line_text in msg.content.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {line_text}"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(""));
            }
            Role::Tool => {
                // Tool messages: check if it's a "thinking" block or a visible tool
                if msg.content.starts_with("thinking (") {
                    // Grouped thinking block — dim, collapsed style
                    let first_line = msg.content.lines().next().unwrap_or("");
                    lines.push(Line::from(Span::styled(
                        format!("  ◆ {first_line}"),
                        Style::default().fg(Color::Rgb(100, 100, 100)),
                    )));
                    for line_text in msg.content.lines().skip(1) {
                        lines.push(Line::from(Span::styled(
                            format!("  {line_text}"),
                            Style::default().fg(Color::Rgb(80, 80, 80)),
                        )));
                    }
                    lines.push(Line::from(""));
                } else if msg.content.contains("--- a/") || msg.content.contains("+++ b/") || msg.content.contains("--- /dev/null") {
                    // Diff output — render with colors
                    let tool_header = msg.content.lines().next().unwrap_or("");
                    lines.push(Line::from(Span::styled(
                        format!("  {tool_header}"),
                        Style::default().fg(Color::Yellow),
                    )));
                    let rest: String = msg.content.lines().skip(1).collect::<Vec<_>>().join("\n");
                    let rendered = render::render_message(&rest, "  ");
                    for line in rendered {
                        let wrapped = wrap_line(line, inner_width);
                        lines.extend(wrapped);
                    }
                    lines.push(Line::from(""));
                } else {
                    // Regular tool output
                    let tool_header = msg.content.lines().next().unwrap_or("");
                    lines.push(Line::from(Span::styled(
                        format!("  {tool_header}"),
                        Style::default().fg(Color::Yellow),
                    )));
                    for line_text in msg.content.lines().skip(1) {
                        lines.push(Line::from(Span::styled(
                            format!("  {line_text}"),
                            Style::default().fg(Color::Rgb(140, 140, 140)),
                        )));
                    }
                    lines.push(Line::from(""));
                }
            }
        }
    }

    // Live thinking indicator — show pending tools while loading
    if app.loading && !app.pending_tools.is_empty() {
        let elapsed = app.request_started
            .map(|s| {
                let d = s.elapsed();
                if d.as_secs() >= 60 {
                    format!("{}m {}s", d.as_secs() / 60, d.as_secs() % 60)
                } else {
                    format!("{}.{}s", d.as_secs(), d.subsec_millis() / 100)
                }
            })
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::styled(
                " merc ",
                Style::default().fg(Color::Black).bg(Color::Rgb(80, 60, 120)).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("thinking... ({} calls, {})", app.pending_tools.len(), elapsed),
                Style::default().fg(Color::Rgb(120, 100, 160)),
            ),
        ]));
        for t in &app.pending_tools {
            let icon = if t.result.is_some() { "✓" } else { "…" };
            let style = if t.result.is_some() {
                Style::default().fg(Color::Rgb(80, 80, 80))
            } else {
                Style::default().fg(Color::Rgb(120, 100, 160))
            };
            lines.push(Line::from(Span::styled(
                format!("    {icon} {} {}", t.name, t.args_summary),
                style,
            )));
        }
        lines.push(Line::from(""));
    }

    // Diffusion buffer
    if app.loading && !app.stream_buffer.is_empty() {
        if app.pending_tools.is_empty() {
            // No thinking block shown yet, show the diffusing header
            lines.push(Line::from(vec![
                Span::styled(
                    " merc ",
                    Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("diffusing...", Style::default().fg(Color::Magenta).add_modifier(Modifier::DIM)),
            ]));
        }
        let rendered = render::render_message(&app.stream_buffer, "  ");
        for line in rendered {
            let dimmed = dim_line(line);
            let wrapped = wrap_line(dimmed, inner_width);
            lines.extend(wrapped);
        }
        lines.push(Line::from(""));
    } else if app.loading && app.pending_tools.is_empty() && app.stream_buffer.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                " merc ",
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("thinking...", Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Duration after last response
    if !app.loading {
        if let Some(d) = &app.last_duration {
            let dur_str = if d.as_secs() >= 60 {
                format!("cooked in {}m {}s", d.as_secs() / 60, d.as_secs() % 60)
            } else {
                format!("cooked in {}.{}s", d.as_secs(), d.subsec_millis() / 100)
            };
            lines.push(Line::from(Span::styled(
                format!("  {dur_str}"),
                Style::default().fg(Color::Rgb(60, 60, 60)),
            )));
        }
    }

    // Auto-scroll
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();
    let scroll = if app.chat_scroll == 0 {
        total_lines.saturating_sub(visible_height) as u16
    } else {
        let max_scroll = total_lines.saturating_sub(visible_height) as u16;
        max_scroll.saturating_sub(app.chat_scroll)
    };

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

/// Wrap a styled Line to fit within max_width
fn wrap_line(line: Line<'static>, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 { return vec![line]; }

    let total_len: usize = line.spans.iter().map(|s| s.content.len()).sum();
    if total_len <= max_width { return vec![line]; }

    let mut chars: Vec<(char, Style)> = Vec::new();
    for span in &line.spans {
        for c in span.content.chars() {
            chars.push((c, span.style));
        }
    }

    let mut result: Vec<Line<'static>> = Vec::new();
    let mut pos = 0;
    let mut first = true;

    while pos < chars.len() {
        let width = if first { max_width } else { max_width.saturating_sub(4) };
        let end = (pos + width).min(chars.len());
        let chunk = &chars[pos..end];

        let mut spans: Vec<Span<'static>> = Vec::new();
        if !first {
            spans.push(Span::raw("    ".to_string()));
        }

        let mut current_text = String::new();
        let mut current_style = chunk.first().map(|(_, s)| *s).unwrap_or_default();

        for (c, style) in chunk {
            if *style == current_style {
                current_text.push(*c);
            } else {
                if !current_text.is_empty() {
                    spans.push(Span::styled(current_text.clone(), current_style));
                    current_text.clear();
                }
                current_style = *style;
                current_text.push(*c);
            }
        }
        if !current_text.is_empty() {
            spans.push(Span::styled(current_text, current_style));
        }

        result.push(Line::from(spans));
        pos = end;
        first = false;
    }

    result
}

fn dim_line(line: Line<'static>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line.spans.into_iter().map(|span| {
        Span::styled(span.content.to_string(), span.style.add_modifier(Modifier::DIM))
    }).collect();
    Line::from(spans)
}
