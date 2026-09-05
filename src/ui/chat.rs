// ponytail: ui/chat.rs owns message-list + textarea rendering.
// Pure functions only — take &AppState (or &InputArea), render Frame.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;
use tui_textarea::TextArea;

use crate::app::{AppState, Role};
use crate::theme;

/// Render the message list into `area`. Read-only — does not own the textarea.
pub fn render_messages(f: &mut Frame<'_>, area: Rect, app: &AppState) {
    if app.messages.is_empty() {
        let hint = match &app.status {
            crate::app::Status::Idle => Span::styled(
                "type a prompt, press enter to send",
                Style::default().fg(theme::FG_MUTE),
            ),
            crate::app::Status::Busy => Span::styled("…", Style::default().fg(theme::ACCENT)),
            crate::app::Status::Error(msg) => {
                Span::styled(msg, Style::default().fg(theme::ERROR))
            }
        };
        Paragraph::new(Line::from(hint))
            .render(area, f.buffer_mut());
        return;
    }

    let items: Vec<ListItem<'_>> = app
        .messages
        .iter()
        .map(|m| {
            let (label, color) = match m.role {
                Role::User => ("You", theme::USER),
                Role::Assistant => ("Assistant", theme::ASSIST),
            };
            let header = Span::styled(label, Style::default().fg(color));
            // Streaming assistant message shows a blinking-cursor on the last char.
            // We approximate by leaving the actual blink to the render tick (Phase 5).
            let body = Span::raw(m.text.as_str());
            let lines = vec![Line::from(header), Line::from(body), Line::from("")];
            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items);
    Widget::render(list, area, f.buffer_mut());
}

/// Build a fresh TextArea styled to match kaku.
/// ponytail: returns an owned TextArea so main.rs can hold the editor state
/// across frames (mutate then render). This matches tui-textarea's idiomatic use.
pub fn build_textarea<'a>() -> TextArea<'a> {
    let mut ta = TextArea::new(vec![String::new()]);
    ta.set_block(theme::block(" input "));
    // Yellow cursor = our single accent. Visible whenever the widget is rendered.
    ta.set_cursor_style(Style::default().fg(theme::ACCENT));
    ta.set_cursor_line_style(Style::default());
    ta
}

/// Render the textarea widget into `area`. The textarea IS the widget — caller
/// owns it and feeds keys via ta.input(Input::from(k)) before re-rendering.
/// Note: tui-textarea impls Widget for &TextArea<'_>, not &mut TextArea<'_>,
/// so we borrow immutably for the render call.
pub fn render_input(f: &mut Frame<'_>, area: Rect, ta: &mut TextArea<'_>) {
    Widget::render(&*ta, area, f.buffer_mut());
}
