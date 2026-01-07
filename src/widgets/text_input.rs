use std::ops::{Bound, Range, RangeBounds};

use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget},
};

use crate::palette::PALETTE;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputMode {
    /// Text field is not being interacted with.
    #[default]
    Normal,
    /// Text field is being edited.
    Editing,
}

/// State for the `TextInput` widget.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct TextInput {
    /// Optional placeholder text when input is empty.
    pub placeholder: Option<String>,
    /// Current input mode.
    pub mode: InputMode,
    /// Current character index.
    pub character_index: usize,
    /// The entered text input value.
    pub input: String,
}

impl TextInput {
    /// Create a new `TextInput` with default values.
    #[inline]
    pub const fn new() -> Self {
        Self {
            placeholder: None,
            mode: InputMode::Normal,
            character_index: 0,
            input: String::new(),
        }
    }

    /// Create a new `TextInput` with a placeholder string.
    #[inline]
    pub fn new_with_placeholder<S: Into<String>>(placeholder: S) -> Self {
        Self {
            placeholder: Some(placeholder.into()),
            mode: InputMode::Normal,
            character_index: 0,
            input: String::new(),
        }
    }

    /// Check if the input is currently empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// Get the current character count of the input.
    #[inline]
    #[must_use]
    pub fn character_count(&self) -> usize {
        self.input.chars().count()
    }

    /// Clamp the given position to be within the valid range of the input.
    #[inline]
    #[must_use]
    pub fn clamp_cursor_pos(&self, pos: usize) -> usize {
        pos.clamp(0, self.character_count())
    }

    /// Set the cursor position, clamped to valid range.
    #[inline]
    pub fn set_cursor_pos(&mut self, pos: usize) {
        self.character_index = self.clamp_cursor_pos(pos);
    }

    /// Move the cursor position left by one, clamped to valid range.
    #[inline]
    pub fn move_cursor_left(&mut self) {
        self.set_cursor_pos(self.character_index.saturating_sub(1));
    }

    /// Move the cursor position right by one, clamped to valid range.
    #[inline]
    pub fn move_cursor_right(&mut self) {
        self.set_cursor_pos(self.character_index.saturating_add(1));
    }

    /// Reset the cursor position back to the start.
    #[inline]
    pub fn reset_cursor(&mut self) {
        self.set_cursor_pos(0);
    }

    /// Enter a character into the text input, moving the cursor right.
    #[inline]
    pub fn enter_char(&mut self, c: char) {
        match self.input.char_indices().nth(self.character_index) {
            Some((byte_index, _)) => self.input.insert(byte_index, c),
            None => self.input.push(c),
        }
        self.move_cursor_right();
    }

    /// Remove the character before the cursor position, and move the cursor left.
    #[inline]
    pub fn backspace(&mut self) {
        if self.character_index == 0 {
            return;
        }
        if let Some((byte_index, ch)) = self.input.char_indices().nth(self.character_index - 1) {
            self.input
                .replace_range(byte_index..byte_index + ch.len_utf8(), "");
            self.move_cursor_left();
        }
    }

    /// Remove the word before the cursor position.
    #[inline]
    pub fn word_backspace(&mut self) {
        if self.character_index == 0 {
            return;
        }

        let left_boundary = self.find_left_boundary(self.character_index);

        // Remove the word
        self.input.replace_range(
            self.index_range_as_byte_range(left_boundary..self.character_index),
            "",
        );
        self.set_cursor_pos(left_boundary);
    }

    /// Remove the character after the cursor position.
    #[inline]
    pub fn del(&mut self) {
        if let Some((byte_index, ch)) = self
            .input
            .char_indices()
            .nth(self.character_index.saturating_add(1))
        {
            self.input
                .replace_range(byte_index..byte_index + ch.len_utf8(), "");
        }
    }

    /// Remove the word after the cursor position.
    #[inline]
    pub fn word_del(&mut self) {
        if self.character_index >= self.character_count() {
            return;
        }

        let right_boundary = self.find_right_boundary(self.character_index);

        self.input.replace_range(
            self.index_range_as_byte_range(self.character_index..right_boundary),
            "",
        );
    }

    /// Ctrl + Left arrow key behaviour.
    #[inline]
    pub fn move_cursor_left_word(&mut self) {
        let left_boundary = self.find_left_boundary(self.character_index);
        self.set_cursor_pos(left_boundary);
    }

    /// Ctrl + Right arrow key behaviour.
    #[inline]
    pub fn move_cursor_right_word(&mut self) {
        let right_boundary = self.find_right_boundary(self.character_index);
        self.set_cursor_pos(right_boundary);
    }

    /// Find the next word boundary to the left from a given index.
    #[inline]
    #[must_use]
    pub fn find_left_boundary(&self, from_index: usize) -> usize {
        if from_index == 0 {
            return 0;
        }

        let characters = self.input.chars().collect::<Vec<_>>();
        let mut new_index = from_index;

        let target_boundary = if new_index > 1
            && characters[new_index.saturating_sub(1)].is_word_boundary()
            && characters[new_index.saturating_sub(2)].is_word_boundary()
        {
            new_index = new_index.saturating_sub(1);
            true
        } else {
            if new_index > 0 && characters[new_index.saturating_sub(1)].is_word_boundary() {
                new_index = new_index.saturating_sub(1);
            }
            false
        };

        while new_index > 0
            && characters[new_index.saturating_sub(1)].is_word_boundary() == target_boundary
        {
            new_index = new_index.saturating_sub(1);
        }

        new_index
    }

    /// Find the next word boundary to the right from a given index.
    #[inline]
    #[must_use]
    pub fn find_right_boundary(&self, from_index: usize) -> usize {
        if from_index >= self.character_count() {
            return self.character_count();
        }

        let characters = self.input.chars().collect::<Vec<_>>();
        let char_count = characters.len();

        let mut new_index = from_index;

        // Mirror word_del: decide whether we are skipping whitespace or non-whitespace.
        let target_boundary = if new_index < char_count.saturating_sub(1)
            && characters[new_index].is_word_boundary()
            && characters[new_index.saturating_add(1)].is_word_boundary()
        {
            new_index = new_index.saturating_add(1);
            true
        } else {
            if new_index < char_count && characters[new_index].is_word_boundary() {
                new_index = new_index.saturating_add(1);
            }
            false
        };

        while new_index < char_count && characters[new_index].is_word_boundary() == target_boundary
        {
            new_index = new_index.saturating_add(1);
        }

        new_index
    }

    /// Convert a character index range to a byte index range.
    #[inline]
    #[must_use]
    pub fn index_range_as_byte_range<R: RangeBounds<usize>>(&self, range: R) -> Range<usize> {
        let start_index = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.saturating_add(1).min(self.character_count()),
            Bound::Unbounded => 0,
        };
        let end_index = match range.end_bound() {
            Bound::Included(&i) => i.saturating_add(1).min(self.character_count()),
            Bound::Excluded(&i) => i,
            Bound::Unbounded => self.input.chars().count(),
        };

        let start_byte_index = self
            .input
            .char_indices()
            .nth(start_index)
            .map_or(0, |(byte_index, _)| byte_index);
        let end_byte_index = self
            .input
            .char_indices()
            .nth(end_index)
            .map_or(self.input.len(), |(byte_index, _)| byte_index);
        start_byte_index..end_byte_index
    }

    /// Clear the text input.
    #[inline]
    pub fn clear(&mut self) {
        self.input.clear();
        self.reset_cursor();
    }
}

impl TextInput {
    /// Handle an event for the text input.
    pub fn handle_event(&mut self, event: &Event) {
        if let Event::Key(key_event) = event
            && self.mode == InputMode::Editing
            && key_event.kind == KeyEventKind::Press
        {
            let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
            let shift = key_event.modifiers.contains(KeyModifiers::SHIFT);
            match key_event.code {
                KeyCode::Char('v' | 'V') if ctrl => {}
                KeyCode::Char(c) => self.enter_char(c),
                KeyCode::Backspace if ctrl && !shift => self.word_backspace(),
                KeyCode::Backspace if ctrl && shift => self.clear(),
                KeyCode::Backspace => self.backspace(),
                KeyCode::Delete if ctrl => self.word_del(),
                KeyCode::Delete => self.del(),
                KeyCode::Enter | KeyCode::Esc => self.mode = InputMode::Normal,
                KeyCode::Left if ctrl => self.move_cursor_left_word(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right if ctrl => self.move_cursor_right_word(),
                KeyCode::Right => self.move_cursor_right(),
                _ => {}
            }
        }
    }

    pub fn render(&self, area: Rect, frame: &mut ratatui::Frame<'_>) {
        let input_empty = self.input.is_empty();
        let display_text = if input_empty && let Some(placeholder) = self.placeholder.as_ref() {
            placeholder.as_str()
        } else {
            self.input.as_str()
        };
        let input = Paragraph::new(display_text).style(match self.mode {
            InputMode::Normal => Style::default().fg(PALETTE.std_fg).dim(),
            InputMode::Editing => {
                let mut style = Style::default().fg(PALETTE.log_warn_fg);
                if input_empty {
                    style = style.dim().fg(PALETTE.std_fg);
                }
                style
            }
        });

        Widget::render(input, area, frame.buffer_mut());

        #[allow(clippy::cast_possible_truncation)]
        if self.mode == InputMode::Editing {
            frame.set_cursor_position(ratatui::layout::Position::new(
                area.x + self.character_index as u16,
                area.y,
            ));
        }
    }
}

/// Characters considered as word boundaries for word-wise operations.
pub const WORD_BOUNDARY_CHARS: &[char] = &[
    '.', ',', ';', ':', '!', '?', '-', '_', '/', '\\', '|', '(', ')', '[', ']', '{', '}', '<', '>',
    '"', '\'',
];

/// Check if a character is considered a word boundary character.
#[inline]
#[must_use]
pub fn is_word_boundary_character(c: char) -> bool {
    c.is_whitespace() || WORD_BOUNDARY_CHARS.contains(&c)
}

/// Extension trait for `char` to check for word boundary characters.
pub trait WordBoundaryCharExt {
    /// Check if the character is a word boundary character.
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn is_word_boundary(self) -> bool;
}
impl WordBoundaryCharExt for char {
    #[inline]
    fn is_word_boundary(self) -> bool {
        is_word_boundary_character(self)
    }
}
impl WordBoundaryCharExt for &char {
    #[inline]
    fn is_word_boundary(self) -> bool {
        is_word_boundary_character(*self)
    }
}
