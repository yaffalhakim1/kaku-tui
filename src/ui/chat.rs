// ponytail: ui/chat.rs owns message-list + textarea rendering.
// Pure functions only — take &AppState (or &InputArea), render Frame.
//
// kaku-style: no borders, edge-to-edge text (column 0 for body, no left
// margin). Role prefixes are chevrons — user uses `>` (low-key shell
// prompt), assistant uses `▶` (heavier marker, like kaku's tab bar).
// Hairline dividers between sections come from the caller (ui/mod.rs).

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Widget};
use ratatui::Frame;
use tui_textarea::TextArea;

use crate::app::{AppState, Role};
use crate::theme;

/// Render a horizontal hairline across the full width of `area`.
/// ponytail: visible section separator in the kaku style — dim, single row.
pub fn render_hairline(f: &mut Frame<'_>, area: Rect) {
    let line = "─".repeat(area.width as usize);
    let p = Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(theme::FG_MUTE),
    )));
    Widget::render(p, area, f.buffer_mut());
}

/// Render the message list into `area`. Read-only.
pub fn render_messages(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if app.messages.is_empty() {
        // Empty state: just leave the area blank. The input prompt below IS the
        // hint — kaku's TUI does the same.
        return;
    }

    let items: Vec<ListItem<'_>> = app
        .messages
        .iter()
        .map(|m| {
            // User prompts get a low-key shell-style `>`. Assistant text gets
            // no chevron — Claude Code's panel doesn't prefix assistant
            // responses, just plain text on multiple lines.
            // User is one rendered line; assistant text wraps naturally.
            match m.role {
                Role::User => ListItem::new(Line::from(vec![
                    Span::styled("› ", Style::default().fg(theme::USER)),
                    Span::raw(&m.text),
                ])),
                Role::Assistant => ListItem::new(Line::from(Span::raw(&m.text))),
            }
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
    ta.set_block(ratatui::widgets::Block::default());
    ta.set_cursor_style(Style::default().fg(theme::ACCENT));
    ta.set_cursor_line_style(Style::default());
    ta
}

/// Render the input prompt at the bottom.
/// Textarea is borderless; we draw the `› ` glyph ourselves in the
/// preceding columns so the user sees a shell-prompt shape.
pub fn render_input(f: &mut Frame<'_>, area: Rect, ta: &mut TextArea<'_>, placeholder: &str) {
    // Split the input row into: [prompt glyph] [textarea].
    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Length(2), // "› "
            ratatui::layout::Constraint::Min(1),
        ])
        .split(area);

    // Prompt: `› ` in kaku tan.
    let prompt = Paragraph::new(Line::from(Span::styled(
        "› ",
        Style::default().fg(theme::ACCENT),
    )));
    Widget::render(prompt, cols[0], f.buffer_mut());

    // Empty-state hint.
    let is_empty = ta.lines().iter().all(|l| l.is_empty());
    if is_empty {
        let hint = Paragraph::new(Line::from(Span::styled(
            placeholder,
            Style::default().fg(theme::FG_MUTE),
        )));
        Widget::render(hint, cols[1], f.buffer_mut());
    }

    // tui-textarea impls Widget for &TextArea<'_>, not &mut, so borrow imm.
    Widget::render(&*ta, cols[1], f.buffer_mut());
}
