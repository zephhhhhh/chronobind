use std::{fmt::Display, ops::Deref};

use const_format::concatcp;
use ratatui::style::Color;

pub use colours::*;

use crate::terminal::BETTER_SYMBOLS;

#[cfg(feature = "better-colours")]
mod colours {
    use ratatui::style::Color;

    // Definitions..
    const DARK_SLATE: Color = Color::Rgb(22, 31, 31);
    const SELECTED_GREEN: Color = Color::Rgb(37, 128, 48);
    const SPECIAL_WHITE: Color = Color::Rgb(205, 232, 250);

    // UI colours..
    pub const HOVER_BG: Color = DARK_SLATE;
    pub const SELECTED_FG: Color = SELECTED_GREEN;
    pub const SPECIAL_FG: Color = SPECIAL_WHITE;

    pub const STD_FG: Color = Color::White;
    pub const STD_BG: Color = Color::Black;

    // Log level colours..
    pub const LOG_ERROR_FG: Color = Color::Rgb(230, 0, 0);
    pub const LOG_WARN_FG: Color = Color::Rgb(249, 241, 105);
    pub const LOG_INFO_FG: Color = Color::Rgb(36, 114, 200);
    pub const LOG_DEBUG_FG: Color = Color::Rgb(77, 166, 235);
    pub const LOG_TRACE_FG: Color = Color::Rgb(204, 204, 204);

    // WoW class colours..
    pub const UNKNOWN_COL: Color = Color::Rgb(225, 225, 225);
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

    // Dedication colours..
    pub const HEART_FG: Color = Color::Rgb(186, 117, 170);
}

#[cfg(not(feature = "better-colours"))]
mod colours {
    use ratatui::style::Color;

    // UI Colours..
    pub const HOVER_BG: Color = Color::Indexed(235);
    pub const SELECTED_FG: Color = Color::Indexed(29);
    pub const SPECIAL_FG: Color = Color::Indexed(189);

    pub const STD_FG: Color = Color::White;
    pub const STD_BG: Color = Color::Black;

    // Log level colours..
    pub const LOG_ERROR_FG: Color = Color::Red;
    pub const LOG_WARN_FG: Color = Color::Yellow;
    pub const LOG_INFO_FG: Color = Color::Blue;
    pub const LOG_DEBUG_FG: Color = Color::Cyan;
    pub const LOG_TRACE_FG: Color = Color::Gray;

    // WoW Class Colours..
    pub const UNKNOWN_COL: Color = Color::Indexed(255);
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

    // Dedication colours..
    pub const HEART_FG: Color = Color::Indexed(139);
}

// Icons, formatting, etc..

/// A pair of symbols, in the form `(better_symbol, normal_symbol)`.
pub struct DualSymbols(pub &'static str, pub &'static str);
impl DualSymbols {
    /// Get the appropriate symbol based on the `BETTER_SYMBOLS` setting.
    #[inline]
    #[must_use]
    pub fn get(&self) -> &'static str {
        if *BETTER_SYMBOLS { self.0 } else { self.1 }
    }
}
impl Deref for DualSymbols {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        if *BETTER_SYMBOLS { &self.0 } else { &self.1 }
    }
}
impl Display for DualSymbols {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// Icon representing the enter key.
pub const ENTER_SYMBOL: DualSymbols = DualSymbols("↵", "Enter");

/// Icon representing a collapsed item.
pub const COLLAPSED_ICON: &str = "▶";
/// Icon representing a collapsed item.
pub const EXPANDED_ICON: &str = "▼";

/// Icon used to indicate scrolling up.
pub const SCROLL_UP_ICON: &str = "↑";
/// Icon used to indicate scrolling down.
pub const SCROLL_DOWN_ICON: &str = "↓";

/// Symbol used to highlight selected items.
pub const HIGHLIGHT_SYMBOL: &str = ">";
/// Reverse highlight symbol used to indicate selection.
pub const HIGHLIGHT_SYMBOL_REVERSED: &str = "<";

/// Symbol used to indicate selected items.
pub const SELECTED_SYMBOL: DualSymbols = DualSymbols("✓", "X");
/// Symbol used to indicate unselected items.
pub const UNSELECTED_SYMBOL: &str = " ";

/// Indentation string for nested items.
/// Represents 1 level of indentation.
pub const INDENTATION_STR: &str = " ";

/// Symbol used to indicate pinned items.
pub const PINNED_SYMBOL: DualSymbols = DualSymbols("☆", "**");

/// Symbol used to indicate unlimited values.
pub const UNLIMITED_SYMBOL: DualSymbols = DualSymbols("∞", "inf");

/// Icon representing an addon file.
pub const ADDON_FILE_ICON: DualSymbols = DualSymbols("📦", "■");
/// Icon representing a config file.
pub const CONFIG_FILE_ICON: DualSymbols = DualSymbols("⚙ ", "≡");

/// Get a string indicating whether an item is pinned, followed by a space if pinned.
#[inline]
#[must_use]
pub fn pinned_string(pinned: bool) -> &'static str {
    const PINNED_BETTER: &str = concatcp!(PINNED_SYMBOL.0, " ");
    const PINNED_STANDARD: &str = concatcp!(PINNED_SYMBOL.1, " ");

    if pinned {
        if *BETTER_SYMBOLS {
            PINNED_BETTER
        } else {
            PINNED_STANDARD
        }
    } else {
        ""
    }
}

/// Get a checkbox string based on whether the item is selected.
#[inline]
#[must_use]
pub fn checkbox(selected: bool) -> &'static str {
    const UNSELECTED: &str = concatcp!('[', UNSELECTED_SYMBOL, ']');
    const SELECTED_BETTER: &str = concatcp!('[', SELECTED_SYMBOL.0, ']');
    const SELECTED_STANDARD: &str = concatcp!('[', SELECTED_SYMBOL.1, ']');

    if selected {
        if *BETTER_SYMBOLS {
            SELECTED_BETTER
        } else {
            SELECTED_STANDARD
        }
    } else {
        UNSELECTED
    }
}

/// Get the highlight symbol based on whether the item is highlighted.
/// # Returns
/// The highlight symbol followed by a space if highlighted, otherwise an empty string.
#[inline]
#[must_use]
pub const fn highlight_symbol(highlighted: bool) -> &'static str {
    const SYMBOL_WITH_SPACE: &str = concatcp!(HIGHLIGHT_SYMBOL, " ");
    if highlighted { SYMBOL_WITH_SPACE } else { "" }
}

/// Get the reversed highlight symbol based on whether the item is highlighted.
/// # Returns
/// The highlight symbol followed by a space if highlighted, otherwise an empty string.
#[inline]
#[must_use]
pub const fn highlight_symbol_rev(highlighted: bool) -> &'static str {
    const SYMBOL_WITH_SPACE: &str = concatcp!(" ", HIGHLIGHT_SYMBOL_REVERSED);
    if highlighted { SYMBOL_WITH_SPACE } else { "" }
}

/// Get text wrapped with highlight symbols if highlighted.
#[inline]
#[must_use]
pub fn highlight_str(text: impl AsRef<str>, highlighted: bool) -> String {
    format!("{}{}", highlight_symbol(highlighted), text.as_ref())
}

/// Get text wrapped with highlight symbols if highlighted.
#[inline]
#[must_use]
pub fn dual_highlight_str(text: impl AsRef<str>, highlighted: bool) -> String {
    format!(
        "{}{}{}",
        highlight_symbol(highlighted),
        text.as_ref(),
        highlight_symbol_rev(highlighted)
    )
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
    INDENTATION_STR.repeat(indent_level)
}

/// Format a `DateTime<Local>` for display in the UI.
#[must_use]
pub fn display_backup_time(dt: &chrono::DateTime<chrono::Local>) -> String {
    dt.format(crate::backend::DISPLAY_TIME_FORMAT).to_string()
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
