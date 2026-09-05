// ponytail: ui is pure. Takes &AppState (+ mut input), renders Frame. No I/O.
//
// kaku-style: no borders anywhere. Top row carries the model tag (left)
// and the key-hint line (right). Below that is the message list, then the
// input prompt line, then the status line.

pub mod chat;
pub mod status;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use tui_textarea::TextArea;

/// Top-level dispatcher. Called from main loop on every tick + event.
pub fn draw<'a>(f: &mut Frame<'_>, app: &AppState, ta: &mut TextArea<'a>) {
    // Vertical layout:
    //   [top tag line]       — 1 row, no chrome
    //   [messages]           — fills the rest
    //   [input prompt]       — 1 row
    //   [status line]        — 1 row
    //
    // ponytail: 4 rows fixed at the bottom; messages get Min(1).
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top tag
            Constraint::Min(1),    // messages
            Constraint::Length(1), // input prompt
            Constraint::Length(1), // status
        ])
        .split(f.area());

    render_top_tag(f, chunks[0], app);
    chat::render_messages(f, chunks[1], app);
    chat::render_input(f, chunks[2], ta, "type your message and press enter to send…");
    status::render(f, chunks[3], app);
}

/// Top tag: model name on the left, key hints on the right, both dim.
/// ponytail: a single row split horizontally. Min() for the left side
/// pushes the right side to the actual right edge.
fn render_top_tag(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(28),
        ])
        .split(area);

    // Left: `kaku-tui · <model>`
    let model = app.default_model.as_deref().unwrap_or("(no model)");
    let left_text = Line::from(vec![
        Span::styled(" kaku-tui ", Style::default().fg(theme::FG_MUTE)),
        Span::styled("· ", Style::default().fg(theme::FG_FAINT)),
        Span::styled(model, Style::default().fg(theme::FG)),
    ]);
    Paragraph::new(left_text).render(cols[0], f.buffer_mut());

    // Right: key hints
    let right = Paragraph::new(Line::from(Span::styled(
        "esc:quit  ⌘k palette",
        Style::default().fg(theme::FG_MUTE),
    )))
    .alignment(ratatui::layout::Alignment::Right);
    right.render(cols[1], f.buffer_mut());
}
