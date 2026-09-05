// Exact colors from ~/.config/opencode/themes/kaku.json (One Dark).
// Ponytail: this is the user's active opencode theme (see tui.json -> "theme":"kaku").
// Keep these hex values in sync with that file — they are the source of truth.
// If you change the palette there, update the `palette` module below only.

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

/// Raw `defs` block from kaku.json. Named 1:1 with that file.
pub mod palette {
    use ratatui::style::Color;

    // Backgrounds
    pub const BG: Color = Color::Rgb(0x1e, 0x1e, 0x1e);       // #1e1e1e
    pub const BG_LIFT: Color = Color::Rgb(0x2d, 0x2d, 0x2d);   // #2d2d2d
    pub const BG_EDGE: Color = Color::Rgb(0x33, 0x33, 0x33);   // #333333
    pub const BG_SUBTLE: Color = Color::Rgb(0x2a, 0x2a, 0x2a); // #2a2a2a

    // Foregrounds
    pub const FG: Color = Color::Rgb(0xd4, 0xd4, 0xd4);       // #d4d4d4
    pub const FG_BRIGHT: Color = Color::Rgb(0xe8, 0xe8, 0xe8); // #e8e8e8
    pub const FG_MUTED: Color = Color::Rgb(0x6e, 0x6e, 0x6e);  // #6e6e6e
    pub const FG_FAINT: Color = Color::Rgb(0x5c, 0x63, 0x70);  // #5c6370

    // Accents
    pub const BLUE: Color = Color::Rgb(0x61, 0xaf, 0xef);     // #61afef
    pub const GREEN: Color = Color::Rgb(0x98, 0xc3, 0x79);    // #98c379
    pub const YELLOW: Color = Color::Rgb(0xe5, 0xc0, 0x7b);   // #e5c07b
    pub const ORANGE: Color = Color::Rgb(0xd1, 0x9a, 0x66);   // #d19a66
    pub const RED: Color = Color::Rgb(0xe0, 0x6c, 0x75);      // #e06c75
    pub const PURPLE: Color = Color::Rgb(0xc6, 0x78, 0xdd);   // #c678dd
    pub const CYAN: Color = Color::Rgb(0x56, 0xb6, 0xc2);     // #56b6c2
    pub const GREY: Color = Color::Rgb(0xab, 0xb2, 0xbf);     // #abb2bf
}

// Semantic roles — mapped from kaku.json's `theme` section.
// Kaku sets primary/secondary/accent all to blue, so blue carries the accent;
// user messages take blue, assistant messages take the bright foreground so
// the two stay legible against each other.

pub const BG: Color = palette::BG;
pub const FG: Color = palette::FG;
pub const FG_DIM: Color = palette::BG_EDGE;       // theme.border
pub const FG_MUTE: Color = palette::FG_MUTED;     // theme.textMuted
pub const FG_BRIGHT: Color = palette::FG_BRIGHT;  // theme.markdownHeading

pub const ACCENT: Color = palette::BLUE;          // theme.accent
pub const USER: Color = palette::BLUE;            // theme.primary
pub const ASSIST: Color = palette::FG_BRIGHT;     // theme.markdownHeading

pub const ERROR: Color = palette::RED;            // theme.error
pub const WARNING: Color = palette::ORANGE;       // theme.warning
pub const SUCCESS: Color = palette::GREEN;        // theme.success
pub const INFO: Color = palette::BLUE;            // theme.info

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
