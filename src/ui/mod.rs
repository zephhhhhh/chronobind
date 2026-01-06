pub mod character;
pub mod main_character_ui;
pub mod messages;

pub use character::{Character, CharacterIndex, CharacterWithIndex, CharacterWithInstall};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// Convert a `KeyCode` to its lowercase equivalent if it's a character.
#[inline]
#[must_use]
pub const fn lower_keycode(keycode: KeyCode) -> KeyCode {
    match keycode {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}

pub trait KeyCodeExt {
    /// Convert the `KeyCode` to its lowercase equivalent if it's a character.
    #[must_use]
    fn keycode_lower(&self) -> KeyCode;
}

impl KeyCodeExt for KeyEvent {
    #[inline]
    fn keycode_lower(&self) -> KeyCode {
        lower_keycode(self.code)
    }
}

/// Truncate a string to a maximum length, appending an ellipsis string if truncated.
#[inline]
#[must_use]
pub fn truncate_with_ellipsis<S: AsRef<str>>(s: S, max_len: usize) -> String {
    const ELLIPSIS: &str = "...";

    let string_ref = s.as_ref();

    // If max_len is too small to even fit `ELLIPSIS`, return a shortened ellipsis
    if max_len <= ELLIPSIS.len() {
        return ".".repeat(max_len);
    }

    // Count characters without allocating
    if string_ref.chars().count() <= max_len {
        return string_ref.to_string();
    }

    // Take (max_len - 3) characters and append `ELLIPSIS`
    let mut truncated = String::with_capacity(max_len);
    truncated.extend(string_ref.chars().take(max_len - ELLIPSIS.len()));
    truncated + ELLIPSIS
}
