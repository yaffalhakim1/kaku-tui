// kaku-style: status is a single dim line at the very bottom. No border,
// no chrome. Just one short state label on the left, key hints on the right.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::{AppState, Status};
use crate::theme;

pub fn render(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(28)])
        .split(area);

    // Left: one short token describing current state.
    let (label, color) = match &app.status {
        Status::Idle if app.abort_requested => ("aborted", theme::ACCENT),
        Status::Idle => ("ready", theme::SUCCESS),
        Status::Busy => ("● streaming", theme::ACCENT),
        Status::Error(msg) => (msg.as_str(), theme::ERROR),
    };
    // Bullet (●) gives Busy a tiny visual cue without being noisy.
    Paragraph::new(Line::from(Span::styled(
        format!(" {label}"),
        Style::default().fg(color),
    )))
    .render(cols[0], f.buffer_mut());

    // Right: context-aware key hints, right-aligned.
    let hint = match &app.status {
        Status::Busy => " esc:abort ",
        _ => " esc:quit ",
    };
    Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(theme::FG_MUTE),
    )))
    .alignment(Alignment::Right)
    .render(cols[1], f.buffer_mut());
}
