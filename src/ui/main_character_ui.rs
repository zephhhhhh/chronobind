use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::InputMode;
use crate::ui::Character;
use crate::widgets::character_list::CharacterListWidget;
use crate::widgets::file_list::{FileListConfig, FileListWidget};

/// Manages the main UI drawing for the application.
#[derive(Debug)]
pub struct MainCharacterUI {
    /// Character list widget for displaying characters.
    pub character_list_widget: CharacterListWidget,
    /// File list widget for displaying character files.
    pub file_list_widget: FileListWidget,
}

impl Default for MainCharacterUI {
    fn default() -> Self {
        Self::new()
    }
}

impl MainCharacterUI {
    /// Create a new `MainUI` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            character_list_widget: CharacterListWidget::new(),
            file_list_widget: FileListWidget::new(),
        }
    }

    /// Draw the entire main UI to the frame.
    pub fn draw(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        characters: &[Character],
        input_mode: InputMode,
        config: &crate::ChronoBindAppConfig,
    ) {
        self.main_screen(area, buf, characters, input_mode, config);
    }

    /// Render the main screen UI.
    fn main_screen(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        characters: &[Character],
        input_mode: InputMode,
        config: &crate::ChronoBindAppConfig,
    ) {
        // Split the main screen into left and right panels
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.character_list(chunks[0], buf, characters);
        self.file_list(chunks[1], buf, characters, input_mode, config);
    }

    /// Render the character list panel.
    fn character_list(&mut self, area: Rect, buf: &mut Buffer, characters: &[Character]) {
        self.character_list_widget.render(area, buf, characters);
    }

    /// Render the file list panel.
    fn file_list(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        characters: &[Character],
        input_mode: InputMode,
        config: &crate::ChronoBindAppConfig,
    ) {
        let char_index = self
            .character_list_widget
            .get_selected_character_index(characters);
        let selected_character = char_index.and_then(|idx| characters.get(idx));
        let show_highlight = input_mode == InputMode::FileSelection;
        let file_list_config = FileListConfig {
            show_friendly_names: config.show_friendly_names,
        };

        self.file_list_widget.render(
            area,
            buf,
            selected_character,
            show_highlight,
            &file_list_config,
        );
    }
}
