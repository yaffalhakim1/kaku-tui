// ponytail: single source of truth for visual tokens. Constants, not args.

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

// Kaku dark — quiet, single accent (warm yellow on near-black).
// Ponytail: never pure white. Gray feels native.
pub const BG: Color = Color::Reset;
pub const FG: Color = Color::Gray;
pub const FG_DIM: Color = Color::DarkGray;
pub const FG_MUTE: Color = Color::Indexed(245);
pub const ACCENT: Color = Color::Yellow;
pub const USER: Color = Color::Cyan;
pub const ASSIST: Color = Color::White;
pub const ERROR: Color = Color::Red;
pub const SUCCESS: Color = Color::Green;

// Spacing — "feels like kaku" = 1 cell padding, never more.
pub const PADDING_H: u16 = 1;
pub const PADDING_V: u16 = 1;

/// Rounded-bordered block with a dim border + plain title.
/// ponytail: this is the only block style. If we need a second one, lift it.
pub fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(FG_DIM))
        .title(title)
        .title_style(Style::default().fg(FG))
}

/// Flat, padded interior for content blocks (no border).
pub fn block_flat() -> Block<'static> {
    Block::default()
        .borders(Borders::NONE)
        .padding(Padding::new(PADDING_H, PADDING_H, PADDING_V, PADDING_V))
}
