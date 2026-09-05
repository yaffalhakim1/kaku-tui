// kaku-style status bar, ported from `config_tui/ui.rs`.
// Three slots left-to-right: state · stats · keybinds footer.
// Keybinds render in kaku's `↑↓ Navigate | Enter Edit | Esc Save` style —
// key tokens in the primary color, action labels dim, pipe separator.
// We collapse to short labels when the terminal is narrow.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::{AppState, Status};
use crate::theme;

const FOOTER_SEP: &str = "  ·  ";
const FOOTER_SHORT_WIDTH: u16 = 52;

#[derive(Clone, Copy)]
struct FooterAction {
    key: &'static str,
    long: &'static str,
    short: &'static str,
}

const IDLE_ACTIONS: [FooterAction; 3] = [
    FooterAction { key: "↑↓", long: "history", short: "hist" },
    FooterAction { key: "⏎", long: "send", short: "send" },
    FooterAction { key: "esc", long: "quit", short: "quit" },
];

const BUSY_ACTIONS: [FooterAction; 3] = [
    FooterAction { key: "⏎", long: "queue", short: "queue" },
    FooterAction { key: "esc", long: "abort", short: "abort" },
    FooterAction { key: "ctrl+c", long: "quit", short: "quit" },
];

const COMMAND_ACTIONS: [FooterAction; 2] = [
    FooterAction { key: "⏎", long: "run", short: "run" },
    FooterAction { key: "esc", long: "clear", short: "clear" },
];

/// The currently-shown footer actions. Switching by mode (idle/busy) and
/// by whether input is empty (command mode shows a different hint).
///
/// ponytail: if the input is non-empty we treat it as "command mode" for
/// hint purposes because we don't yet distinguish user text from
/// `/`-prefixed text in the keybind layer. Cheap, informative.
fn active_actions(app: &AppState) -> &'static [FooterAction] {
    if !app.input.trim().is_empty() {
        return &COMMAND_ACTIONS;
    }
    match app.status {
        Status::Busy => &BUSY_ACTIONS,
        _ => &IDLE_ACTIONS,
    }
}

pub fn render(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // state token
            Constraint::Min(1),    // stats
            Constraint::Min(20),   // footer (keybinds)
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

    // ── Stats — session id shortened + model override indicator ──
    let stats = stats_line(app);
    if !stats.is_empty() {
        Paragraph::new(Line::from(Span::styled(
            stats,
            Style::default().fg(theme::FG_MUTE),
        )))
        .render(cols[1], f.buffer_mut());
    }

    // ── Keybinds footer (kaku config_tui style: `key label | key label`) ──
    let actions = active_actions(app);
    let use_short = cols[2].width < FOOTER_SHORT_WIDTH;
    let line = build_footer_line(actions, use_short, cols[2].width);
    Paragraph::new(line)
        .alignment(Alignment::Right)
        .render(cols[2], f.buffer_mut());
}

fn build_footer_line(actions: &[FooterAction], short: bool, width: u16) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let max_w = width as usize;
    let mut used = 0usize;
    for (idx, a) in actions.iter().enumerate() {
        let label = if short { a.short } else { a.long };
        // `key<space>label` then `  ·  ` separator except before the first.
        let seg_w = a.key.chars().count() + 1 + label.chars().count();
        let sep_w = if idx == 0 { 0 } else { FOOTER_SEP.chars().count() };
        if used + sep_w + seg_w > max_w {
            break;
        }
        if idx > 0 {
            spans.push(Span::styled(FOOTER_SEP, Style::default().fg(theme::FG_FAINT)));
            used += sep_w;
        }
        spans.push(Span::styled(a.key, Style::default().fg(theme::PRIMARY)));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme::FG_MUTE),
        ));
        used += seg_w;
    }
    Line::from(spans)
}

/// Stats summary for the middle slot.
/// ponytail: extend with token counts once we wire StepFinishPart.
fn stats_line(app: &AppState) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = &app.session {
        let id = s.id.strip_prefix("ses_").unwrap_or(&s.id);
        let tail = &id[id.len().saturating_sub(8)..];
        parts.push(format!("ses:{tail}"));
    }
    if let Some(m) = &app.current_model_override {
        parts.push(format!("model:{m}"));
    }
    parts.join("  ·  ")
}
