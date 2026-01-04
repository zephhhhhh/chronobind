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
