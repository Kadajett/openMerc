use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use super::chat::draw_chat;
use super::diff_panel::draw_diff_panel;
use super::input::draw_input;
use super::status::draw_status;

/// Main draw function — lays out all panels
pub fn draw(f: &mut Frame, app: &App) {
    // Determine input box height based on number of lines in the input buffer
    let input_lines = app.input.lines().count();
    // Minimum 1 line, maximum 5 lines (adjust as needed)
    let input_height = std::cmp::min(std::cmp::max(input_lines, 1), 5) as u16;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),       // main content area
            Constraint::Length(input_height), // dynamic input box height
            Constraint::Length(1),     // status bar
        ])
        .split(f.area());

    // Side panel always visible (Ctrl+D to toggle)
    if app.show_diff_panel {
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55),  // chat
                Constraint::Percentage(45),  // side panel (tabbed)
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
