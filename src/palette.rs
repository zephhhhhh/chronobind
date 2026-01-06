use std::sync::LazyLock;
use std::{fmt::Display, ops::Deref};

use const_format::concatcp;
use ratatui::style::Color;

use crate::terminal::{BETTER_COLOURS, BETTER_SYMBOLS};

/// The currently selected palette.
pub static PALETTE: LazyLock<&'static TUIPalette> = LazyLock::new(|| {
    if *BETTER_COLOURS {
        &better_colours::PALETTE
    } else {
        &standard_colours::PALETTE
    }
});

/// Palette of colours used in the TUI.
#[derive(Debug, Clone)]
pub struct TUIPalette {
    /// Background colour when hovering over an item.
    pub hover_bg: Color,
    /// Colour for selected text/items.
    pub selected_fg: Color,
    /// Colour for special text/items.
    pub special_fg: Color,

    /// Standard foreground colour.
    pub std_fg: Color,
    /// Standard inverted foreground colour.
    pub std_fg_invert: Color,
    /// Standard background colour.
    pub std_bg: Color,

    // Log level colours..
    /// Colour for displaying an error message.
    pub log_error_fg: Color,
    /// Colour for displaying a warning message.
    pub log_warn_fg: Color,
    /// Colour for displaying an info message.
    pub log_info_fg: Color,
    /// Colour for displaying a debug message.
    pub log_debug_fg: Color,
    /// Colour for displaying a trace message.
    pub log_trace_fg: Color,

    // WoW class colours..
    /// Colour for displaying an unknown class.
    pub unknown_col: Color,
    pub warrior_col: Color,
    pub paladin_col: Color,
    pub hunter_col: Color,
    pub rogue_col: Color,
    pub priest_col: Color,
    pub deathknight_col: Color,
    pub shaman_col: Color,
    pub mage_col: Color,
    pub warlock_col: Color,
    pub monk_col: Color,
    pub druid_col: Color,
    pub demonhunter_col: Color,
    pub evoker_col: Color,

    // Dedication colours..
    /// Colour for displaying dedication text.
    pub heart_fg: Color,
}

impl TUIPalette {
    /// Get the appropriate foreground colour based on selection state.
    #[inline]
    #[must_use]
    pub const fn selection_fg(&self, selected: bool) -> Color {
        if selected {
            self.selected_fg
        } else {
            self.std_fg
        }
    }

    /// Get the colour associated with a log level.
    #[inline]
    #[must_use]
    pub const fn log_level_colour(&self, level: log::Level) -> Color {
        match level {
            log::Level::Error => self.log_error_fg,
            log::Level::Warn => self.log_warn_fg,
            log::Level::Info => self.log_info_fg,
            log::Level::Debug => self.log_debug_fg,
            log::Level::Trace => self.log_trace_fg,
        }
    }
}

pub mod better_colours {
    use crate::palette::TUIPalette;
    use ratatui::style::Color;

    // Definitions..
    const DARK_SLATE: Color = Color::Rgb(22, 31, 31);
    const SELECTED_GREEN: Color = Color::Rgb(37, 128, 48);
    const SPECIAL_WHITE: Color = Color::Rgb(205, 232, 250);

    pub const PALETTE: TUIPalette = TUIPalette {
        hover_bg: DARK_SLATE,
        selected_fg: SELECTED_GREEN,
        special_fg: SPECIAL_WHITE,

        std_fg: Color::White,
        std_fg_invert: Color::Black,
        std_bg: Color::Reset,

        log_error_fg: Color::Rgb(230, 0, 0),
        log_warn_fg: Color::Rgb(249, 241, 105),
        log_info_fg: Color::Rgb(36, 114, 200),
        log_debug_fg: Color::Rgb(77, 166, 235),
        log_trace_fg: Color::Rgb(204, 204, 204),
        unknown_col: Color::Rgb(225, 225, 225),
        warrior_col: Color::Rgb(198, 155, 109),
        paladin_col: Color::Rgb(244, 140, 186),
        hunter_col: Color::Rgb(170, 211, 144),
        rogue_col: Color::Rgb(255, 244, 104),
        priest_col: Color::Rgb(255, 255, 255),
        deathknight_col: Color::Rgb(196, 30, 58),
        shaman_col: Color::Rgb(0, 112, 221),
        mage_col: Color::Rgb(63, 199, 235),
        warlock_col: Color::Rgb(135, 136, 238),
        monk_col: Color::Rgb(0, 255, 152),
        druid_col: Color::Rgb(255, 124, 10),
        demonhunter_col: Color::Rgb(163, 48, 201),
        evoker_col: Color::Rgb(51, 147, 127),
        heart_fg: Color::Rgb(186, 117, 170),
    };
}

pub mod standard_colours {
    use crate::palette::TUIPalette;
    use ratatui::style::Color;

    pub const PALETTE: TUIPalette = TUIPalette {
        hover_bg: Color::Indexed(235),
        selected_fg: Color::Indexed(29),
        special_fg: Color::Indexed(189),
        std_fg: Color::White,
        std_fg_invert: Color::Black,
        std_bg: Color::Reset,
        log_error_fg: Color::Red,
        log_warn_fg: Color::Yellow,
        log_info_fg: Color::Blue,
        log_debug_fg: Color::Cyan,
        log_trace_fg: Color::Gray,
        unknown_col: Color::Indexed(255),
        warrior_col: Color::Indexed(173),
        paladin_col: Color::Indexed(211),
        hunter_col: Color::Indexed(150),
        rogue_col: Color::Indexed(227),
        priest_col: Color::White,
        deathknight_col: Color::Indexed(161),
        shaman_col: Color::Indexed(26),
        mage_col: Color::Indexed(80),
        warlock_col: Color::Indexed(105),
        monk_col: Color::Indexed(48),
        druid_col: Color::Indexed(208),
        demonhunter_col: Color::Indexed(134),
        evoker_col: Color::Indexed(66),
        heart_fg: Color::Indexed(139),
    };
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
