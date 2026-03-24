use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render a message string into styled Lines, handling:
/// - Diff blocks (colored red/green with preserved prefixes)
/// - Markdown headers (#, ##, ###)
/// - Bold (**text**)
/// - Inline code (`code`)
/// - Code blocks (```...```)
/// - Tables (| col | col |)
pub fn render_message(content: &str, indent: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut in_diff = false;

    for raw_line in content.lines() {
        let line = raw_line;

        // Detect code block boundaries
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                // Check if this is a diff code block
                let after = line.trim_start().strip_prefix("```").unwrap_or("");
                if after.starts_with("diff") {
                    in_diff = true;
                }
                lines.push(Line::from(Span::styled(
                    format!("{indent}{line}"),
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                in_diff = false;
                lines.push(Line::from(Span::styled(
                    format!("{indent}{line}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            continue;
        }

        // Inside code block
        if in_code_block {
            if in_diff || is_diff_line(line) {
                lines.push(render_diff_line(line, indent));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("{indent}{line}"),
                    Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 30)),
                )));
            }
            continue;
        }

        // Detect standalone diff lines (from tool output, not in code block)
        if is_diff_line(line) {
            lines.push(render_diff_line(line, indent));
            continue;
        }

        // Markdown headers
        if line.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &line[4..]),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if line.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &line[3..]),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if line.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &line[2..]),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }

        // Table rows
        if line.starts_with('|') && line.ends_with('|') {
            if line.contains("---") {
                // Separator row
                lines.push(Line::from(Span::styled(
                    format!("{indent}{line}"),
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                lines.push(render_table_row(line, indent));
            }
            continue;
        }

        // Regular text with inline formatting
        lines.push(render_inline_markdown(line, indent));
    }

    lines
}

/// Check if a line looks like a diff line
fn is_diff_line(line: &str) -> bool {
    line.starts_with('+') && !line.starts_with("+++")
        || line.starts_with('-') && !line.starts_with("---")
        || line.starts_with("@@ ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
}

/// Render a single diff line with appropriate coloring
fn render_diff_line<'a>(line: &str, indent: &str) -> Line<'static> {
    let (style, prefix) = if line.starts_with("@@") {
        (Style::default().fg(Color::Cyan), "")
    } else if line.starts_with("+++") || line.starts_with("---") {
        (Style::default().fg(Color::White).add_modifier(Modifier::BOLD), "")
    } else if line.starts_with('+') {
        (Style::default().fg(Color::Green), "")
    } else if line.starts_with('-') {
        (Style::default().fg(Color::Red), "")
    } else if line.starts_with(' ') {
        (Style::default().fg(Color::DarkGray), "")
    } else {
        (Style::default(), "")
    };

    Line::from(Span::styled(
        format!("{indent}{prefix}{line}"),
        style,
    ))
}

/// Render a table row with column highlighting
fn render_table_row<'a>(line: &str, indent: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(indent.to_string())];

    for (i, cell) in line.split('|').enumerate() {
        if i > 0 {
            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        }
        if !cell.is_empty() {
            spans.push(Span::styled(
                cell.to_string(),
                Style::default().fg(Color::White),
            ));
        }
    }

    Line::from(spans)
}

/// Render a line with inline markdown: **bold**, `code`, *italic*
fn render_inline_markdown<'a>(line: &str, indent: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(indent.to_string())];
    let mut chars = line.chars().peekable();
    let mut current = String::new();

    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                // Bold
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }
                chars.next(); // consume second *
                let mut bold_text = String::new();
                while let Some(bc) = chars.next() {
                    if bc == '*' && chars.peek() == Some(&'*') {
                        chars.next();
                        break;
                    }
                    bold_text.push(bc);
                }
                spans.push(Span::styled(
                    bold_text,
                    Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
                ));
            }
            '`' => {
                // Inline code
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }
                let mut code_text = String::new();
                while let Some(cc) = chars.next() {
                    if cc == '`' {
                        break;
                    }
                    code_text.push(cc);
                }
                spans.push(Span::styled(
                    code_text,
                    Style::default().fg(Color::Yellow).bg(Color::Rgb(40, 40, 40)),
                ));
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::raw(current));
    }

    Line::from(spans)
}
