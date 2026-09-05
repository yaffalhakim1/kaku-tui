// Exact colors from kaku-app's `kaku/src/kaku_theme.rs` (dark_palette).
// ponytail: this is the real kaku palette. The `kaku.json` in
// ~/.config/opencode/themes/ uses One Dark instead and is a separate
// opencode-side theme — not the same thing as kaku's actual terminal
// colors. We match the kaku-app side because that's what the
// user sees in their reference screenshot.
//
//            kaku.rs      our theme role
//  primary   #8E6AD9  ->  USER (bold + purple — user prompt label)
//  secondary #58D8AD  ->  SUCCESS (teal — "ok" state)
//  accent    #DAAE76  ->  ACCENT (warm tan — cursor, chevrons)
//  error     #D85D5D  ->  ERROR
//  text      #D5D4D6  ->  FG (body)
//  muted     #6D6D6D  ->  FG_MUTE, FG_DIM
//  bg        #15141B  ->  BG

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

/// Raw kaku palette. Names mirror `dark_palette()` in kaku_theme.rs.
pub mod palette {
    use ratatui::style::Color;

    pub const PRIMARY: Color = Color::Rgb(0x8e, 0x6a, 0xd9); // #8E6AD9 — purple
    pub const SECONDARY: Color = Color::Rgb(0x58, 0xd8, 0xad); // #58D8AD — teal
    pub const ACCENT: Color = Color::Rgb(0xda, 0xae, 0x76); // #DAAE76 — warm tan
    pub const ERROR: Color = Color::Rgb(0xd8, 0x5d, 0x5d); // #D85D5D — red
    pub const TEXT: Color = Color::Rgb(0xd5, 0xd4, 0xd6); // #D5D4D6 — off-white
    pub const MUTED: Color = Color::Rgb(0x6d, 0x6d, 0x6d); // #6D6D6D — gray
    pub const BG: Color = Color::Rgb(0x15, 0x14, 0x1b); // #15141B — near-black purple
}

// ── Semantic roles ──
// Maps semantic intent to kaku palette tokens. The top tag and dim hints
// use `palette::MUTED` for the secondary text; the accent (chevron, cursor)
// uses the warm tan instead of the purple so the user prompt label and
// the prompt glyph read distinctly.

pub const BG: Color = palette::BG;
pub const FG: Color = palette::TEXT;
pub const FG_DIM: Color = palette::MUTED;
pub const FG_MUTE: Color = palette::MUTED;
pub const FG_FAINT: Color = palette::MUTED;
pub const FG_BRIGHT: Color = palette::TEXT;

pub const ACCENT: Color = palette::ACCENT;
pub const USER: Color = palette::PRIMARY;
pub const ASSIST: Color = palette::TEXT;

pub const ERROR: Color = palette::ERROR;
pub const WARNING: Color = palette::ACCENT;
pub const SUCCESS: Color = palette::SECONDARY;
pub const INFO: Color = palette::PRIMARY;

// Spacing — kaku is low-density: 1 cell margin, never more.
pub const PADDING_H: u16 = 1;
pub const PADDING_V: u16 = 1;

/// Rounded-bordered block with a dim border + plain title.
/// ponytail: still unused in the kaku redesign (no borders), but kept
/// for tests and any future use.
#[allow(dead_code)]
pub fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(FG_DIM))
        .title(title)
        .title_style(Style::default().fg(FG))
}

/// Flat, padded interior for content blocks (no border).
#[allow(dead_code)]
pub fn block_flat() -> Block<'static> {
    Block::default()
        .borders(Borders::NONE)
        .padding(Padding::new(PADDING_H, PADDING_H, PADDING_V, PADDING_V))
}
