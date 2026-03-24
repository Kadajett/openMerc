use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use super::chat::draw_chat;
use super::diff_panel::draw_diff_panel;
use super::input::draw_input;
use super::status::draw_status;

/// Main draw function — lays out all panels
pub fn draw(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),       // main content area
            Constraint::Length(3),     // input box
            Constraint::Length(1),     // status bar
        ])
        .split(f.area());

    // If diff panel is visible and there are modified files, split horizontally
    if app.show_diff_panel && !app.modified_files.is_empty() {
        let hsplit = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(55),  // chat
                Constraint::Percentage(45),  // diff panel
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
