use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;

/// Draw the diff panel showing modified files and their diffs
pub fn draw_diff_panel(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 10 || area.height < 5 {
        return;
    }

    let block = Block::default()
        .title(" Changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    if app.modified_files.is_empty() {
        let msg = Paragraph::new("No files modified yet")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, area);
        return;
    }

    // Split: file list at top (3 lines per file + border), diff below
    let file_list_height = (app.modified_files.len() as u16 + 2).min(area.height / 3);

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(file_list_height),
            ratatui::layout::Constraint::Min(3),
        ])
        .split(area);

    // File list
    let items: Vec<ListItem> = app.modified_files.iter().enumerate().map(|(i, fd)| {
        let style = if i == app.diff_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };
        let prefix = if i == app.diff_selected { ">" } else { " " };
        ListItem::new(Line::from(Span::styled(
            format!("{prefix} {}", fd.path),
            style,
        )))
    }).collect();

    let file_list = List::new(items)
        .block(Block::default().title(" Modified Files ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
    f.render_widget(file_list, chunks[0]);

    // Diff content for selected file
    if let Some(fd) = app.modified_files.get(app.diff_selected) {
        let mut lines: Vec<Line> = Vec::new();

        for line_text in fd.diff.lines() {
            let (style, text) = if line_text.starts_with('+') && !line_text.starts_with("+++") {
                (Style::default().fg(Color::Green), line_text)
            } else if line_text.starts_with('-') && !line_text.starts_with("---") {
                (Style::default().fg(Color::Red), line_text)
            } else if line_text.starts_with("@@") {
                (Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), line_text)
            } else if line_text.starts_with("---") || line_text.starts_with("+++") {
                (Style::default().fg(Color::White).add_modifier(Modifier::BOLD), line_text)
            } else {
                // Context lines — try to syntax highlight
                let highlighted = highlight_code_line(line_text, &fd.path);
                lines.push(highlighted);
                continue;
            };

            lines.push(Line::from(Span::styled(text.to_string(), style)));
        }

        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let total = lines.len();
        let scroll = if app.diff_scroll == 0 {
            total.saturating_sub(visible_height) as u16
        } else {
            let max = total.saturating_sub(visible_height) as u16;
            max.saturating_sub(app.diff_scroll)
        };

        let diff_view = Paragraph::new(lines)
            .block(Block::default().title(format!(" {} ", fd.path)).borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
            .scroll((scroll, 0));
        f.render_widget(diff_view, chunks[1]);
    }
}

/// Basic syntax highlighting for a code line based on file extension
fn highlight_code_line<'a>(line: &str, path: &str) -> Line<'a> {
    let is_rust = path.ends_with(".rs");
    let is_toml = path.ends_with(".toml");

    if !is_rust && !is_toml {
        return Line::from(Span::styled(line.to_string(), Style::default().fg(Color::DarkGray)));
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if !indent.is_empty() {
        spans.push(Span::raw(indent.to_string()));
    }

    if is_rust {
        // Rust keywords
        let keywords = ["fn ", "pub ", "let ", "mut ", "use ", "mod ", "impl ", "struct ", "enum ",
                        "match ", "if ", "else ", "for ", "while ", "return ", "async ", "await",
                        "self", "Self", "super", "crate", "where", "trait ", "type "];
        let mut remaining = trimmed.to_string();

        if remaining.starts_with("//") {
            spans.push(Span::styled(remaining, Style::default().fg(Color::DarkGray)));
        } else if remaining.starts_with("#[") {
            spans.push(Span::styled(remaining, Style::default().fg(Color::Yellow)));
        } else {
            let mut found_kw = false;
            for kw in &keywords {
                if remaining.starts_with(kw) {
                    spans.push(Span::styled(kw.to_string(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
                    spans.push(Span::styled(remaining[kw.len()..].to_string(), Style::default().fg(Color::White)));
                    found_kw = true;
                    break;
                }
            }
            if !found_kw {
                spans.push(Span::styled(remaining, Style::default().fg(Color::White)));
            }
        }
    } else if is_toml {
        if trimmed.starts_with('[') {
            spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
        } else if trimmed.starts_with('#') {
            spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::DarkGray)));
        } else if let Some(eq_pos) = trimmed.find('=') {
            spans.push(Span::styled(trimmed[..eq_pos].to_string(), Style::default().fg(Color::Yellow)));
            spans.push(Span::styled("=".to_string(), Style::default().fg(Color::White)));
            spans.push(Span::styled(trimmed[eq_pos+1..].to_string(), Style::default().fg(Color::Green)));
        } else {
            spans.push(Span::styled(trimmed.to_string(), Style::default().fg(Color::White)));
        }
    }

    Line::from(spans)
}
