// ponytail: ui/chat.rs owns message-list + textarea rendering.
// Pure functions only — take &AppState (or &InputArea), render Frame.
//
// kaku-style: no borders, no padding frames. Text goes edge-to-edge with a
// 2-cell left margin. Role prefixes are chevrons (▶). The input prompt is a
// single bottom line that looks like a shell prompt (›).

use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Widget};
use ratatui::Frame;
use tui_textarea::TextArea;

use crate::app::{AppState, Role};
use crate::theme;

// ── Layout constants ──
// ponytail: these are the only chrome knobs we have. If we ever need to
// nudge margin/indent, edit here, not in callers.
const LEFT_MARGIN: u16 = 2;

/// Left-margin-adjusted area helper. Everything we draw lives inside this.
fn indented(area: Rect) -> Rect {
    area.inner(Margin::new(LEFT_MARGIN, 0))
}

/// Render the message list into `area`. Read-only.
pub fn render_messages(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    let area = indented(area);

    if app.messages.is_empty() {
        // Empty state: just leave the area blank. The input prompt below IS the
        // hint — kaku's TUI does the same.
        return;
    }

    let items: Vec<ListItem<'_>> = app
        .messages
        .iter()
        .map(|m| {
            // Kaku-style chevron prefix: ▶ for the role marker, role name in
            // a brighter shade. User prompts in blue, assistant in white.
            let chevron = match m.role {
                Role::User => (">>>", theme::ACCENT),
                Role::Assistant => ("▶", theme::ASSIST),
            };
            let chev = Span::styled(chevron.0, Style::default().fg(chevron.1));
            let label_color = match m.role {
                Role::User => theme::USER,
                Role::Assistant => theme::ASSIST,
            };
            let label = Span::styled(
                match m.role {
                    Role::User => "You",
                    Role::Assistant => "Assistant",
                },
                Style::default().fg(label_color).add_modifier(
                    ratatui::style::Modifier::BOLD,
                ),
            );
            // Layout per message:
            //   ▶ You
            //     explain ratatui streaming
            //   <blank>
            let header = Line::from(vec![chev, Span::raw(" "), label]);
            let body = Line::from(Span::raw(&m.text));
            let blank = Line::from("");
            ListItem::new(vec![header, body, blank])
        })
        .collect();

    let list = List::new(items);
    Widget::render(list, area, f.buffer_mut());
}

/// Build a fresh TextArea.
/// ponytail: returns an owned TextArea so main.rs can hold the editor state
/// across frames (mutate then render). Matches tui-textarea's idiomatic use.
pub fn build_textarea<'a>() -> TextArea<'a> {
    let mut ta = TextArea::new(vec![String::new()]);
    // No border — kaku's input is just a prompt line on the bg.
    ta.set_block(ratatui::widgets::Block::default());
    // Accent cursor.
    ta.set_cursor_style(Style::default().fg(theme::ACCENT));
    ta.set_cursor_line_style(Style::default());
    ta
}

/// Render the input prompt at the bottom.
/// The textarea itself is borderless; we draw the `› ` glyph and a hint copy
/// ourselves in the same row, so the user sees a shell-prompt shape.
pub fn render_input(f: &mut Frame<'_>, area: Rect, ta: &mut TextArea<'_>, placeholder: &str) {
    let area = indented(area);

    // Split the input row into: [prompt glyph] [textarea].
    // The textarea gets the rest of the row; we render the `›` separately.
    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length(2),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(area);

    // Prompt: `›` in the accent.
    let prompt = Paragraph::new(Line::from(Span::styled(
        "› ",
        Style::default().fg(theme::ACCENT),
    )));
    Widget::render(prompt, cols[0], f.buffer_mut());

    // If the textarea is empty, show the placeholder in dim text behind it.
    let is_empty = ta.lines().iter().all(|l| l.is_empty());
    if is_empty {
        // Draw placeholder at column 0; the textarea's cursor will overlay it.
        let hint = Paragraph::new(Line::from(Span::styled(
            placeholder,
            Style::default().fg(theme::FG_MUTE),
        )));
        Widget::render(hint, cols[1], f.buffer_mut());
    }

    // ponytail: tui-textarea impls Widget for &TextArea<'_>, not &mut TextArea<'_>,
    // so we borrow immutably for the render call. Mutating input before this point
    // (via ta.input(Input::from(k))) does NOT need a re-borrow.
    Widget::render(&*ta, cols[1], f.buffer_mut());
}
