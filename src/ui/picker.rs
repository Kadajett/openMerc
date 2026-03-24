use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Alignment};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// Draw the session picker screen
pub fn draw_session_picker(f: &mut Frame, sessions: &[String], selected: usize) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Min(5),    // session list
            Constraint::Length(2), // help
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " openMerc ",
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" — Select a session"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Session list
    let mut items: Vec<ListItem> = sessions.iter().enumerate().map(|(i, title)| {
        let style = if i == selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if i == selected { "▶ " } else { "  " };
        ListItem::new(Line::from(Span::styled(
            format!("{prefix}{title}"),
            style,
        )))
    }).collect();

    // Add "New Session" option at the end
    let new_style = if selected == sessions.len() {
        Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let new_prefix = if selected == sessions.len() { "▶ " } else { "  " };
    items.push(ListItem::new(Line::from(Span::styled(
        format!("{new_prefix}+ New Session"),
        new_style,
    ))));

    let list = List::new(items)
        .block(Block::default().title(" Sessions ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    f.render_widget(list, chunks[1]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("navigate  "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("select  "),
        Span::styled(" n ", Style::default().fg(Color::Yellow)),
        Span::raw("new session  "),
        Span::styled(" q ", Style::default().fg(Color::Yellow)),
        Span::raw("quit"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}
