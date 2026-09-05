// kaku-style status bar: 3 segments left-to-right — state · stats · hints.
// Matches Claude Code's `Context 0% | Sh: 10% (1k57o)` style of dense
// meta info packed into the bottom row.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::{AppState, Status};
use crate::theme;

pub fn render(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    // Three segments: state (left) · stats (middle) · hints (right).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // state token
            Constraint::Min(1),    // stats (token counts, etc.)
            Constraint::Length(28), // hints
        ])
        .split(area);

    // ── State token ──
    let (state_text, state_color) = match &app.status {
        Status::Idle if app.abort_requested => ("aborted", theme::ACCENT),
        Status::Idle => ("● ready", theme::SUCCESS),
        Status::Busy => ("● streaming", theme::ACCENT),
        Status::Error(msg) => (msg.as_str(), theme::ERROR),
    };
    Paragraph::new(Line::from(Span::styled(
        format!(" {state_text}"),
        Style::default().fg(state_color),
    )))
    .render(cols[0], f.buffer_mut());

    // ── Stats — only meaningful when we have session info. v0 has nothing
    //    to report beyond session id, so show session id shortened. ponytail:
    //    when we wire token counts from the SSE event, surface them here.
    let stats = stats_line(app);
    if !stats.is_empty() {
        Paragraph::new(Line::from(Span::styled(
            stats,
            Style::default().fg(theme::FG_MUTE),
        )))
        .render(cols[1], f.buffer_mut());
    }

    // ── Hints ──
    let hint = match &app.status {
        Status::Busy => " esc:abort ",
        _ => " esc:quit ",
    };
    Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(theme::FG_MUTE),
    )))
    .alignment(Alignment::Right)
    .render(cols[2], f.buffer_mut());
}

/// Cheap stats summary for the status bar.
/// ponytail: extend this when we wire token counts.
fn stats_line(app: &AppState) -> String {
    let Some(s) = &app.session else {
        return String::new();
    };
    // Show short session id (last 8 chars). Real token counts come from
    // a future wire-up of part metadata.
    let id = s.id.strip_prefix("ses_").unwrap_or(&s.id);
    let tail = &id[id.len().saturating_sub(8)..];
    format!("session:{tail}")
}
