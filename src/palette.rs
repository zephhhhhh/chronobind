use const_format::concatcp;
use ratatui::style::Color;

pub use colours::*;

#[cfg(feature = "better-colours")]
mod colours {
    use ratatui::style::Color;

    // Definitions..
    const DARK_SLATE: Color = Color::Rgb(22, 31, 31);
    const SELECTED_GREEN: Color = Color::Rgb(37, 128, 48);

    const SPECIAL_WHITE: Color = Color::Rgb(205, 232, 250);

    // UI Colours..
    pub const HOVER_BG: Color = DARK_SLATE;
    pub const SELECTED_FG: Color = SELECTED_GREEN;
    pub const SPECIAL_FG: Color = SPECIAL_WHITE;
    pub const STD_FG: Color = Color::White;

    // Log level colours..
    pub const LOG_ERROR_FG: Color = Color::Rgb(230, 0, 0);
    pub const LOG_WARN_FG: Color = Color::Rgb(249, 241, 105);
    pub const LOG_INFO_FG: Color = Color::Rgb(36, 114, 200);
    pub const LOG_DEBUG_FG: Color = Color::Rgb(77, 166, 235);
    pub const LOG_TRACE_FG: Color = Color::Rgb(204, 204, 204);

    // WoW Class Colours..
    pub const UNKNOWN_COL: Color = Color::Rgb(130, 130, 130);
    pub const WARRIOR_COL: Color = Color::Rgb(198, 155, 109);
    pub const PALADIN_COL: Color = Color::Rgb(244, 140, 186);
    pub const HUNTER_COL: Color = Color::Rgb(170, 211, 144);
    pub const ROGUE_COL: Color = Color::Rgb(255, 244, 104);
    pub const PRIEST_COL: Color = Color::Rgb(255, 255, 255);
    pub const DEATHKNIGHT_COL: Color = Color::Rgb(196, 30, 58);
    pub const SHAMAN_COL: Color = Color::Rgb(0, 112, 221);
    pub const MAGE_COL: Color = Color::Rgb(63, 199, 235);
    pub const WARLOCK_COL: Color = Color::Rgb(135, 136, 238);
    pub const MONK_COL: Color = Color::Rgb(0, 255, 152);
    pub const DRUID_COL: Color = Color::Rgb(255, 124, 10);
    pub const DEMONHUNTER_COL: Color = Color::Rgb(163, 48, 201);
    pub const EVOKER_COL: Color = Color::Rgb(51, 147, 127);
}

#[cfg(not(feature = "better-colours"))]
mod colours {
    use ratatui::style::Color;

    // UI Colours..
    pub const HOVER_BG: Color = Color::Indexed(235);
    pub const SELECTED_FG: Color = Color::Indexed(29);
    pub const SPECIAL_FG: Color = Color::Indexed(189);
    pub const STD_FG: Color = Color::White;

    // Log level colours..
    pub const LOG_ERROR_FG: Color = Color::Red;
    pub const LOG_WARN_FG: Color = Color::Yellow;
    pub const LOG_INFO_FG: Color = Color::Blue;
    pub const LOG_DEBUG_FG: Color = Color::Cyan;
    pub const LOG_TRACE_FG: Color = Color::Gray;

    // WoW Class Colours..
    pub const UNKNOWN_COL: Color = Color::Indexed(8);
    pub const WARRIOR_COL: Color = Color::Indexed(173);
    pub const PALADIN_COL: Color = Color::Indexed(211);
    pub const HUNTER_COL: Color = Color::Indexed(150);
    pub const ROGUE_COL: Color = Color::Indexed(227);
    pub const PRIEST_COL: Color = Color::White;
    pub const DEATHKNIGHT_COL: Color = Color::Indexed(161);
    pub const SHAMAN_COL: Color = Color::Indexed(26);
    pub const MAGE_COL: Color = Color::Indexed(80);
    pub const WARLOCK_COL: Color = Color::Indexed(105);
    pub const MONK_COL: Color = Color::Indexed(48);
    pub const DRUID_COL: Color = Color::Indexed(208);
    pub const DEMONHUNTER_COL: Color = Color::Indexed(134);
    pub const EVOKER_COL: Color = Color::Indexed(66);
}

// Icons, formatting, etc..

/// Icon representing a collapsed item.
pub const COLLAPSED_ICON: &str = "▶";
/// Icon representing a collapsed item.
pub const EXPANDED_ICON: &str = "▼";

/// Symbol used to highlight selected items.
pub const HIGHLIGHT_SYMBOL: &str = ">";

/// Get the highlight symbol based on whether the item is highlighted.
/// # Returns
/// The highlight symbol followed by a space if highlighted, otherwise an empty string.
#[inline]
#[must_use]
pub const fn highlight_symbol(highlighted: bool) -> &'static str {
    const SYMBOL_WITH_SPACE: &str = concatcp!(HIGHLIGHT_SYMBOL, " ");
    if highlighted { SYMBOL_WITH_SPACE } else { "" }
}

/// Get the expandable icon based on collapsed state.
#[inline]
#[must_use]
pub const fn expandable_icon(collapsed: bool) -> &'static str {
    if collapsed {
        COLLAPSED_ICON
    } else {
        EXPANDED_ICON
    }
}

/// Indentation string for nested items.
#[inline]
#[must_use]
pub fn indentation(indent_level: usize) -> String {
    " ".repeat(indent_level)
}

/// Convert an (r, g, b) tuple into a `Color::Rgb`
#[inline]
#[must_use]
pub const fn into_colour((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

/// Get the colour associated with a log level.
#[inline]
#[must_use]
pub const fn log_level_colour(level: log::Level) -> Color {
    match level {
        log::Level::Error => LOG_ERROR_FG,
        log::Level::Warn => LOG_WARN_FG,
        log::Level::Info => LOG_INFO_FG,
        log::Level::Debug => LOG_DEBUG_FG,
        log::Level::Trace => LOG_TRACE_FG,
    }
}
