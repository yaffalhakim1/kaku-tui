// ponytail: ui is pure. Takes &AppState (+ mut input), renders Frame. No I/O.
//
// kaku-style: no borders. Edge-to-edge text. Hairline dividers between
// sections (top tag / header band / body / input prompt / status).
// Five vertical slots with single-row dividers between them.

pub mod chat;
pub mod status;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::app::AppState;
use crate::theme;
use tui_textarea::TextArea;

/// Top-level dispatcher.
pub fn draw<'a>(f: &mut Frame<'_>, app: &AppState, ta: &mut TextArea<'a>) {
    // Vertical layout:
    //   [top tag]            1 row
    //   [hairline]           1 row
    //   [header band]        1 row
    //   [hairline]           1 row
    //   [messages]           fills rest
    //   [hairline]           1 row
    //   [input prompt]       1 row
    //   [hairline]           1 row
    //   [status]             1 row
    //
    // ponytail: the hairlines are what make this read as kaku. Drop them
    // and the layout collapses into one undifferentiated block.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top tag
            Constraint::Length(1), // hairline
            Constraint::Length(1), // header band
            Constraint::Length(1), // hairline
            Constraint::Min(1),    // messages
            Constraint::Length(1), // hairline
            Constraint::Length(1), // input prompt
            Constraint::Length(1), // hairline
            Constraint::Length(1), // status
        ])
        .split(f.area());

    render_top_tag(f, chunks[0], app);
    chat::render_hairline(f, chunks[1]);
    render_header_band(f, chunks[2], app);
    chat::render_hairline(f, chunks[3]);
    chat::render_messages(f, chunks[4], app);
    chat::render_hairline(f, chunks[5]);
    chat::render_input(f, chunks[6], ta, "type your message and press enter to send…");
    chat::render_hairline(f, chunks[7]);
    status::render(f, chunks[8], app);
}

/// Top tag: `kaku-tui · <model>` on the left, hint shortcuts on the right.
/// Same row, dim color. Matches kaku's tab-bar style of `Kaku on › <thing>`.
fn render_top_tag(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(28)])
        .split(area);

    let model = app.default_model.as_deref().unwrap_or("(no model)");
    let left = Line::from(vec![
        Span::styled(" kaku ", Style::default().fg(theme::FG_MUTE)),
        Span::styled("› ", Style::default().fg(theme::ACCENT)),
        Span::styled(model, Style::default().fg(theme::FG)),
    ]);
    Paragraph::new(left).render(cols[0], f.buffer_mut());

    let right = Paragraph::new(Line::from(vec![
        Span::styled("esc", Style::default().fg(theme::PRIMARY)),
        Span::styled(" quit  ", Style::default().fg(theme::FG_MUTE)),
        Span::styled("/", Style::default().fg(theme::PRIMARY)),
        Span::styled(" cmds", Style::default().fg(theme::FG_MUTE)),
    ]))
    .alignment(ratatui::layout::Alignment::Right);
    right.render(cols[1], f.buffer_mut());
}

/// Header band: brand + meta line (model, state).
/// One row, dim color. Underneath the top tag, separated by a hairline,
/// it acts as a "this is what we are" strip — like Claude Code's
/// "Claude Code v2.1.104 / Sonnet 4.6 · Claude Max" band.
fn render_header_band(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let model = app.default_model.as_deref().unwrap_or("");
    let state = match &app.status {
        crate::app::Status::Idle if app.abort_requested => ("aborted", theme::ACCENT),
        crate::app::Status::Idle => ("● ready", theme::SUCCESS),
        crate::app::Status::Busy => ("● streaming", theme::ACCENT),
        crate::app::Status::Error(_) => ("● error", theme::ERROR),
    };
    let sep = Span::styled("  ·  ", Style::default().fg(theme::FG_FAINT));

    let mut spans = vec![
        Span::styled(" kaku-tui ", Style::default().fg(theme::FG)),
        Span::styled("v0.1.0", Style::default().fg(theme::FG_MUTE)),
        sep.clone(),
        Span::styled(state.0, Style::default().fg(state.1)),
    ];
    if !model.is_empty() {
        spans.push(sep.clone());
        spans.push(Span::styled(model, Style::default().fg(theme::FG)));
    }
    let line = Line::from(spans);
    Paragraph::new(line).render(area, f.buffer_mut());
}
