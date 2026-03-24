use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FocusPanel};

pub fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == FocusPanel::Input {
        Color::Green
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" > ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let input = Paragraph::new(app.input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(input, area);

    // Show cursor
    if app.focus == FocusPanel::Input {
        f.set_cursor_position((
            area.x + app.cursor_pos as u16 + 1,
            area.y + 1,
        ));
    }
}
