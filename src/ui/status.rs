use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{AppState, Status};
use crate::theme;

pub fn render(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let label = match &app.status {
        Status::Idle if app.abort_requested => ("aborted", theme::ACCENT),
        Status::Idle => ("ready", theme::SUCCESS),
        Status::Busy => ("streaming…", theme::ACCENT),
        Status::Error(msg) => (msg.as_str(), theme::ERROR),
    };
    let left = Paragraph::new(format!(" {}", label.0)).style(Style::default().fg(label.1));
    let hint = match &app.status {
        Status::Busy => "esc:abort  enter:send ",
        _ => "esc:quit  enter:send ",
    };
    let right = Paragraph::new(hint).style(Style::default().fg(theme::FG_DIM));
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(20),
        ])
        .split(area);
    f.render_widget(left, chunks[0]);
    f.render_widget(right, chunks[1]);
}
