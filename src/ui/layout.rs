use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use super::chat::draw_chat;
use super::input::draw_input;
use super::status::draw_status;

/// Main draw function — lays out all panels
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),       // chat area
            Constraint::Length(3),     // input box
            Constraint::Length(1),     // status bar
        ])
        .split(f.area());

    draw_chat(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);
}
