// ponytail: ui is pure. Takes &AppState (+ mut input), renders Frame. No I/O.

pub mod chat;
pub mod status;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use tui_textarea::TextArea;

/// Top-level dispatcher. Called from main loop on every tick + event.
pub fn draw<'a>(f: &mut Frame<'_>, app: &AppState, ta: &mut TextArea<'a>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),       // messages
            Constraint::Length(3),    // input — 3 rows = border + 1 line + border
            Constraint::Length(1),    // status
        ])
        .split(f.area());

    // Outer block: title at top spans full height. We give it only chunk[0] but
    // extend by drawing through the chunks we actually own.
    let title = match &app.session {
        Some(s) => format!(" kaku-tui · {} ", s.title),
        None => " kaku-tui ".to_string(),
    };
    let outer = theme::block(&title);
    // ponytail: we don't render `outer` into the whole frame — instead render
    // into each chunk's left edge via a Column-with-margins approach is overkill.
    // Simpler: render outer over chunk[0] only, since the visual chrome lives there.
    ratatui::widgets::Widget::render(outer, chunks[0], f.buffer_mut());

    let inner = chunks[0].inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    chat::render_messages(f, inner, app);

    chat::render_input(f, chunks[1], ta);
    status::render(f, chunks[2], app);
}
