use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use super::chat::draw_chat;
use super::diff_panel::draw_diff_panel;
use super::input::draw_input;
use super::status::draw_status;

/// Main draw function — lays out all panels
pub fn draw(f: &mut Frame, app: &App) {
    let term_width = f.area().width;

    // Dynamic input height: grows with content, min 3 lines, max 8 (6 visible + borders)
    let content_lines = if term_width > 2 {
        let inner_w = (term_width as usize).saturating_sub(2);
        app.input.lines()
            .map(|line| if line.is_empty() { 1 } else { (line.len() + inner_w - 1) / inner_w })
            .sum::<usize>()
            .max(1)
    } else {
        1
    };
    let input_height = (content_lines as u16 + 2).clamp(3, 8); // +2 for borders

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),                    // chat + side panel
            Constraint::Length(input_height),       // input box (dynamic, 3-8 lines)
            Constraint::Length(1),                  // status bar (always at bottom)
        ])
        .split(f.area());

    // Side panel (Ctrl+D to toggle)
    if app.show_diff_panel {
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55),
                Constraint::Percentage(45),
            ])
            .split(main_chunks[0]);

        draw_chat(f, app, hsplit[0]);
        draw_diff_panel(f, app, hsplit[1]);
    } else {
        draw_chat(f, app, main_chunks[0]);
    }

    draw_input(f, app, main_chunks[1]);
    draw_status(f, app, main_chunks[2]);
}
