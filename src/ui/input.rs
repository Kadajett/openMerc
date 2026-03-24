use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;

    // Calculate how many visual lines the input takes with wrapping
    let visual_lines = if inner_width > 0 {
        app.input.lines()
            .map(|line| {
                if line.is_empty() { 1 } else { (line.len() + inner_width - 1) / inner_width }
            })
            .sum::<usize>()
            .max(1)
    } else {
        1
    };

    // Scroll: if content exceeds visible area, scroll to keep cursor visible
    let scroll_offset = if visual_lines > inner_height {
        (visual_lines - inner_height) as u16
    } else {
        0
    };

    let input = Paragraph::new(app.input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    f.render_widget(input, area);

    // Show cursor — account for wrapping
    if app.focus == FocusPanel::Input {
        let cursor_line = if inner_width > 0 {
            app.cursor_pos / inner_width
        } else {
            0
        };
        let cursor_col = if inner_width > 0 {
            app.cursor_pos % inner_width
        } else {
            0
        };
        // Adjust cursor for scroll
        let visible_line = cursor_line as u16 - scroll_offset.min(cursor_line as u16);

        if visible_line < inner_height as u16 {
            f.set_cursor_position((
                area.x + cursor_col as u16 + 1,
                area.y + visible_line + 1,
            ));
        }
    }
}
