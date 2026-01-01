#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod backend;
pub mod files;
pub mod palette;
pub mod tui_log;
pub mod widgets;
pub mod wow;

use widgets::character_list::{CharacterListWidget, NavigationAction};
use widgets::file_list::{FileListConfig, FileListWidget, FileSelectionAction};
use widgets::popup::PopupState;

#[allow(clippy::wildcard_imports)]
use palette::*;

use std::path::PathBuf;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::Context;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

use crate::wow::WowCharacter;

/// Entry point..
fn main() -> Result<()> {
    // Bootstrap better panic handling..
    color_eyre::install()?;

    // Initialize logging that binds to our TUI..
    tui_log::init_tui_logger(log::LevelFilter::Debug);

    let mut app = ChronoBindApp::new();
    let mut terminal = ratatui::init();

    if set_console_window_title("ChronoBind").is_err() {
        log::warn!("Failed to set console window title");
    }

    let result = app.run(&mut terminal);

    ratatui::restore();

    result
}

/// Set the console window title.
/// # Errors
/// Returns an error if writing to stdout fails.
fn set_console_window_title(title: &str) -> crate::files::AnyResult<()> {
    use std::io::{Write, stdout};
    let mut stdout = stdout();
    write!(stdout, "\x1b]0;{title}\x07")?;
    stdout.flush()?;

    Ok(())
}

pub enum PopupKind {
    BackupPopup,
}

pub struct PopupInfo {
    pub kind: PopupKind,
    pub items: Vec<String>,
}

/// Representation of a `WoW` character along with its selected files and
/// options inside the app UI.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Character {
    /// The underlying `WoW` character data.
    pub character: WowCharacter,
    /// Which config files are selected.
    selected_config_files: Vec<bool>,
    /// Which addon files are selected.
    selected_addon_files: Vec<bool>,
    /// Whether the addon options section is collapsed.
    addon_options_collapsed: bool,
}

impl Character {
    #[must_use]
    pub fn new(character: &WowCharacter) -> Self {
        let config_file_count = character.config_files.len();
        let addon_file_count = character.addon_files.len();
        Self {
            character: character.clone(),
            selected_config_files: vec![false; config_file_count],
            selected_addon_files: vec![false; addon_file_count],
            addon_options_collapsed: false,
        }
    }

    /// Get the display name of the character, optionally including the realm.
    #[must_use]
    pub fn display_name(&self, show_realm: bool) -> String {
        if show_realm {
            format!("{} - {}", self.name(), self.realm())
        } else {
            self.name().to_string()
        }
    }

    /// Get the realm of the character.
    #[inline]
    #[must_use]
    pub fn realm(&self) -> &str {
        &self.character.realm
    }

    /// Get the name of the character.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.character.name
    }

    /// Get the config files of the character.
    #[inline]
    #[must_use]
    pub fn config_files(&self) -> &[wow::WowCharacterFile] {
        &self.character.config_files
    }

    /// Get the addon files of the character.
    #[inline]
    #[must_use]
    pub fn addon_files(&self) -> &[wow::WowCharacterFile] {
        &self.character.addon_files
    }

    /// Check if a config file at the given index is selected.
    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_config_file_selected(&self, index: usize) -> bool {
        self.selected_config_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Toggle the selected status of a config file at the given index.
    #[inline]
    pub fn toggle_config_file_selected(&mut self, index: usize) -> bool {
        self.selected_config_files
            .get_mut(index)
            .is_some_and(|selected| {
                *selected = !*selected;
                *selected
            })
    }

    /// Check if an addon file at the given index is selected.
    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_addon_file_selected(&self, index: usize) -> bool {
        self.selected_addon_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Toggle the selected status of an addon file at the given index.
    #[inline]
    pub fn toggle_addon_file_selected(&mut self, index: usize) -> bool {
        self.selected_addon_files
            .get_mut(index)
            .is_some_and(|selected| {
                *selected = !*selected;
                *selected
            })
    }

    /// Check if any config files are selected.
    #[inline]
    #[must_use]
    pub fn any_config_file_selected(&self) -> bool {
        self.selected_config_files.iter().any(|&s| s)
    }

    /// Check if any addon file is selected.
    #[inline]
    #[must_use]
    pub fn any_addon_file_selected(&self) -> bool {
        self.selected_addon_files.iter().any(|&s| s)
    }

    /// Check if any files (regular or addon) are selected.
    #[inline]
    #[must_use]
    pub fn any_file_selected(&self) -> bool {
        self.any_config_file_selected() || self.any_addon_file_selected()
    }

    /// Check if all config files are selected.
    #[inline]
    #[must_use]
    pub fn all_config_files_selected(&self) -> bool {
        self.selected_config_files.iter().all(|&s| s)
    }

    /// Check if all addon files are selected.
    #[inline]
    #[must_use]
    pub fn all_addon_files_selected(&self) -> bool {
        self.selected_addon_files.iter().all(|&s| s)
    }

    /// Set the selected status of all config files.
    #[inline]
    pub fn set_all_config_selected(&mut self, state: bool) {
        self.selected_config_files.fill(state);
    }

    /// Set the selected status of all addon files.
    #[inline]
    pub fn set_all_addon_selected(&mut self, state: bool) {
        self.selected_addon_files.fill(state);
    }

    #[inline]
    pub fn set_all_selected(&mut self, state: bool) {
        self.set_all_config_selected(state);
        self.set_all_addon_selected(state);
    }

    /// Get all selected files from both config files and addon files.
    #[must_use]
    pub fn get_all_selected_files(&self) -> Vec<PathBuf> {
        let mut selected_paths = Vec::new();

        for (i, selected) in self.selected_config_files.iter().enumerate() {
            if *selected && let Some(file) = self.config_files().get(i) {
                selected_paths.push(file.get_full_filename().into());
            }
        }

        for (i, selected) in self.selected_addon_files.iter().enumerate() {
            if *selected && let Some(file) = self.addon_files().get(i) {
                let path = PathBuf::from(wow::SAVED_VARIABLES).join(file.get_full_filename());
                selected_paths.push(path);
            }
        }

        selected_paths
    }
}

/// Application configuration options.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChronoBindAppConfig {
    /// Whether to show realm names alongside character names.
    pub show_realm: bool,
    /// Whether to show friendly names for files instead of raw filenames.
    pub show_friendly_names: bool,
    /// Whether to operate in mock mode (no actual file operations).
    pub mock_mode: bool,
}

/// Different input modes for the application.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum InputMode {
    /// Navigating the character list.
    #[default]
    Navigation,
    /// Selecting files for a character.
    FileSelection,
    /// Interacting with a popup menu.
    Popup,
}

/// Main application state.
#[derive(Debug, Default)]
pub struct ChronoBindApp {
    /// Application configuration settings.
    config: ChronoBindAppConfig,

    /// Whether the application should exit.
    should_exit: bool,

    /// Currently selected `WoW` branch identifier.
    selected_branch: Option<String>,
    /// Located `WoW` installations.
    wow_installations: Vec<wow::WowInstall>,
    /// List of characters across the selected branch.
    characters: Vec<Character>,
    /// Index of the character from which files were copied.
    copied_char: Option<usize>,

    /// Current input mode of the application.
    input_mode: InputMode,

    /// Whether to show the console debug output.
    show_console: bool,
    /// Scroll offset for the debug console.
    debug_scroll_offset: usize,

    /// Character list widget for displaying characters.
    character_list_widget: CharacterListWidget,
    /// File list widget for displaying character files.
    file_list_widget: FileListWidget,

    /// Current popup state, if any.
    popup: Option<PopupState>,
}

impl ChronoBindApp {
    #[must_use]
    pub fn new() -> Self {
        // Sample WoW characters with their associated files
        let wow_installs = match wow::locate_wow_installs() {
            Ok(installs) => installs,
            Err(e) => {
                log::error!("Failed to locate WoW installations: {e}");
                Vec::new()
            }
        };

        let mut app = Self {
            config: ChronoBindAppConfig {
                show_realm: false,
                show_friendly_names: true,
                mock_mode: true,
            },
            should_exit: false,

            selected_branch: None,
            wow_installations: wow_installs,
            characters: Vec::new(),
            copied_char: None,

            input_mode: InputMode::Navigation,

            show_console: false,
            debug_scroll_offset: 0,

            character_list_widget: CharacterListWidget::new(),
            file_list_widget: FileListWidget::new(),

            popup: None,
        };

        app.load_branch_characters(wow::WOW_RETAIL_IDENT);

        app
    }
}

impl ChronoBindApp {
    /// Find a `WoW` installation by its branch identifier.
    #[inline]
    #[must_use]
    pub fn find_wow_branch(&self, branch: &str) -> Option<&wow::WowInstall> {
        self.wow_installations
            .iter()
            .find(|install| install.branch_ident.to_lowercase() == branch.to_lowercase())
    }

    /// Get the currently selected character index.
    #[inline]
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.character_list_widget.selected_index()
    }

    /// Get the actual character index from `selected_index`, accounting for grouped display
    fn get_selected_character_index(&self) -> Option<usize> {
        self.character_list_widget
            .get_selected_character_index(&self.characters)
    }

    /// Get the currently selected branch's `WoW` installation.
    #[inline]
    #[must_use]
    pub fn get_selected_branch_install(&self) -> Option<&wow::WowInstall> {
        self.find_wow_branch(self.selected_branch.as_ref()?)
    }

    /// Get the character with its associated install for the character at the given index.
    #[inline]
    #[must_use]
    pub fn character_with_install(&self, index: usize) -> Option<(&Character, &wow::WowInstall)> {
        let character = self.characters.get(index)?;
        let install = self.find_wow_branch(&character.character.branch)?;
        Some((character, install))
    }

    /// Get the `WoW` installation for the character at the given index.
    #[inline]
    #[must_use]
    pub fn get_wow_branch_for_character(&self, index: usize) -> Option<&wow::WowInstall> {
        let character = self.characters.get(index)?;
        self.find_wow_branch(&character.character.branch)
    }

    /// Refresh the backups for the character at the given index.
    pub fn refresh_character_backups(&mut self, index: usize) -> bool {
        let Some(install) = self.get_wow_branch_for_character(index).cloned() else {
            return false;
        };
        let Some(character) = self.characters.get_mut(index) else {
            return false;
        };
        character.character.refresh_backups(&install)
    }

    /// Refresh the backups for the character at the given index.
    pub fn refresh_character(&mut self, index: usize) -> bool {
        let Some(install) = self.get_wow_branch_for_character(index).cloned() else {
            return false;
        };
        let Some(character) = self.characters.get_mut(index) else {
            return false;
        };
        character.character.refresh_character_info(&install)
    }

    /// Load the characters from a given `WoW` branch identifier.
    pub fn load_branch_characters(&mut self, branch: &str) {
        self.characters.clear();
        self.character_list_widget.state.select(Some(0));
        self.copied_char = None;

        let Some(install) = self.find_wow_branch(branch) else {
            log::error!("No WoW installation found for branch: {branch}");
            return;
        };

        let Some(characters) = install
            .find_all_characters_and_files()
            .map(|chars| chars.iter().map(Character::new).collect::<Vec<_>>())
        else {
            log::error!(
                "Failed to find characters in installation at {}",
                install.install_path
            );
            return;
        };

        self.characters = characters;
        self.selected_branch = Some(branch.to_string());
    }
}

impl ChronoBindApp {
    /// Open a popup menu with the given state.
    pub fn open_popup(&mut self, popup: PopupState) {
        self.popup = Some(popup);
        self.input_mode = InputMode::Popup;
    }

    /// Runs the main application loop.
    /// # Errors
    /// Returns an error if event polling or reading fails.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        match wow::locate_wow_installs() {
            Ok(installs) => {
                log::info!("Located {} WoW installations:", installs.len());
                for install in installs {
                    log::info!(
                        "{} at {}",
                        install.display_branch_name(),
                        install.install_path
                    );
                    let Some(characters) = install.find_all_characters() else {
                        log::warn!(
                            "Failed to find accounts in installation at {}",
                            install.install_path
                        );
                        continue;
                    };

                    for character in characters {
                        log::info!(
                            " - Character: {} - {} / {}",
                            character.name,
                            character.realm,
                            character.account
                        );
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to locate WoW installations: {e}");
            }
        }

        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// Draw the entire application UI.
    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.top_bar(chunks[0], frame.buffer_mut());
        self.main_screen(chunks[1], frame.buffer_mut());
        self.bottom_bar(chunks[2], frame.buffer_mut());

        // Render popup on top if it's open
        if self.input_mode == InputMode::Popup {
            let popup_area = popup_area(frame.area(), 35, 30);
            self.render_popup(popup_area, frame.buffer_mut());
        }
    }

    /// Handle key down events.
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char('r') => {
                log::debug!("Refreshing character list..");
                if let Some(branch) = self.selected_branch.clone() {
                    self.load_branch_characters(&branch);
                } else {
                    log::warn!("No branch selected to refresh characters");
                }
                log::debug!("Character list refreshed.");
            }
            KeyCode::F(1) => {
                self.show_console = !self.show_console;
            }
            KeyCode::F(2) => {
                self.config.show_friendly_names = !self.config.show_friendly_names;
            }
            KeyCode::Char('q') => {
                log::debug!("Quit requested");
                self.should_exit = true;
            }
            _ => {}
        }

        if self.show_console {
            self.handle_console_output_keys(key);
        } else {
            match self.input_mode {
                InputMode::Navigation => self.handle_char_navigation_commands(key),
                InputMode::FileSelection => self.handle_file_selection_commands(key),
                InputMode::Popup => self.handle_popup_keys(key),
            }
        }
    }

    /// Handle scrolling input for the console output.
    const fn handle_console_output_keys(&mut self, key: &KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let speed_multiplier = if ctrl { 3 } else { 1 };
        match key.code {
            KeyCode::Up | KeyCode::Char('w') => {
                self.debug_scroll_offset =
                    self.debug_scroll_offset.saturating_add(speed_multiplier);
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.debug_scroll_offset =
                    self.debug_scroll_offset.saturating_sub(speed_multiplier);
            }
            KeyCode::PageUp => {
                self.debug_scroll_offset = self
                    .debug_scroll_offset
                    .saturating_add(10 * speed_multiplier);
            }
            KeyCode::PageDown => {
                self.debug_scroll_offset = self
                    .debug_scroll_offset
                    .saturating_sub(10 * speed_multiplier);
            }
            KeyCode::Home => {
                self.debug_scroll_offset = 0;
            }
            KeyCode::End => {
                self.debug_scroll_offset = tui_log::TuiLogger::MAX_LOG_SIZE;
            }
            _ => {}
        }
    }

    /// Handle commands from the character navigation widget.
    fn handle_char_navigation_commands(&mut self, key: &KeyEvent) {
        let action = self
            .character_list_widget
            .handle_navigation_input(key, &self.characters);

        match action {
            NavigationAction::None => {}
            NavigationAction::EnterFileSelection => {
                self.input_mode = InputMode::FileSelection;
                self.file_list_widget.state.select(Some(0));
            }
            NavigationAction::ShowBackup(char_idx) => {
                self.show_backup_popup(char_idx);
            }
            NavigationAction::Copy(char_idx) => {
                if let Some(character) = self.characters.get(char_idx)
                    && character.any_file_selected()
                {
                    self.copied_char = Some(char_idx);
                    log::info!("Selected files copied to clipboard");
                } else {
                    log::warn!("No files selected to copy");
                }
            }
            NavigationAction::Paste(target_char_idx) => {
                if let Some(source_char_idx) = self.copied_char {
                    if source_char_idx == target_char_idx {
                        log::warn!(
                            "Cannot paste files onto the same character they were copied from"
                        );
                    } else {
                        let files_to_paste = self.characters[source_char_idx]
                            .get_all_selected_files()
                            .len();
                        self.show_paste_confirm_popup(target_char_idx, files_to_paste);
                    }
                } else {
                    log::warn!("No files copied to paste");
                }
            }
        }
    }

    /// Handle commands from the file selection widget.
    fn handle_file_selection_commands(&mut self, key: &KeyEvent) {
        let char_index = self.get_selected_character_index();
        let character = char_index.and_then(|idx| self.characters.get_mut(idx));

        if let Some(character) = character {
            let action = self
                .file_list_widget
                .handle_file_selection_input(key, character);

            match action {
                FileSelectionAction::None => {}
                FileSelectionAction::ExitFileSelection => {
                    self.input_mode = InputMode::Navigation;
                }
                FileSelectionAction::ShowBackup => {
                    if let Some(char_idx) = self.get_selected_character_index() {
                        self.show_backup_popup(char_idx);
                    }
                }
                FileSelectionAction::Copy => {
                    if let Some(char_idx) = self.get_selected_character_index()
                        && let Some(character) = self.characters.get(char_idx)
                        && character.any_file_selected()
                    {
                        self.copied_char = Some(char_idx);
                        log::info!("Selected files copied to clipboard");
                    } else {
                        log::warn!("No files selected to copy");
                    }
                }
            }
        }
    }

    /// Handle popup menu key events.
    fn handle_popup_keys(&mut self, key: &KeyEvent) {
        if let Some(popup) = &mut self.popup {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.input_mode = InputMode::Navigation;
                    self.popup = None;
                    log::debug!("Closed popup");
                }
                KeyCode::Up | KeyCode::Char('w') => {
                    popup.move_up();
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    popup.move_down();
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.handle_popup_selection();
                }
                _ => {}
            }
        }
    }

    /// Handle the popup selection action.
    fn handle_popup_selection(&mut self) {
        if self.popup.is_none() {
            return;
        }
        let popup = unsafe { &mut *std::ptr::from_mut(self.popup.as_mut().unwrap()) };
        let should_close_popup = popup.handle_selection(self).unwrap_or(true);

        if should_close_popup {
            self.input_mode = InputMode::Navigation;
            self.popup = None;
        }
    }

    /// Handle input events.
    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(250)).context("Event poll failed")? {
            let ev = event::read().context("Event read failed")?;
            if let Event::Key(k) = ev
                && k.kind == KeyEventKind::Press
            {
                self.on_key_down(&k);
            }
        }
        Ok(())
    }
}

// Ui..
impl ChronoBindApp {
    /// Render the main screen UI.
    fn main_screen(&mut self, area: Rect, buf: &mut Buffer) {
        if self.show_console {
            // Split into three sections: characters, files, and debug
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(area);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(main_chunks[0]);

            self.character_list(top_chunks[0], buf);
            self.file_list(top_chunks[1], buf);
            self.console_panel(main_chunks[1], buf);

            return;
        }

        // Split the main screen into left and right panels
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.character_list(chunks[0], buf);
        self.file_list(chunks[1], buf);
    }

    /// Render the character list panel.
    fn character_list(&mut self, area: Rect, buf: &mut Buffer) {
        self.character_list_widget
            .render(area, buf, &self.characters);
    }

    /// Render the file list panel.
    fn file_list(&mut self, area: Rect, buf: &mut Buffer) {
        let char_index = self.get_selected_character_index();
        let selected_character = char_index.and_then(|idx| self.characters.get(idx));
        let show_highlight = self.input_mode == InputMode::FileSelection;
        let config = FileListConfig {
            show_friendly_names: self.config.show_friendly_names,
        };

        self.file_list_widget
            .render(area, buf, selected_character, show_highlight, &config);
    }

    /// Render the console output panel.
    fn console_panel(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::styled(
            " Console Output ",
            Style::default().add_modifier(Modifier::BOLD),
        );

        let block = Block::bordered().title(title).border_set(border::THICK);

        let log_lines: Option<Vec<Line>> = tui_log::with_debug_logs(|logs| {
            let visible_lines = area.height.saturating_sub(2) as usize;
            let total_logs = logs.len();

            let max_scroll = total_logs.saturating_sub(visible_lines);
            self.debug_scroll_offset = self.debug_scroll_offset.min(max_scroll);

            // Get the visible slice of logs starting from scroll_offset
            // Since logs are newest-first, scrolling up shows older logs.
            logs.iter()
                .rev()
                .skip(max_scroll - self.debug_scroll_offset)
                .take(visible_lines)
                .map(|log| {
                    let color = log_level_colour(log.level());
                    Line::from(log.content().to_string()).style(Style::default().fg(color))
                })
                .collect()
        });

        let log_text = log_lines.unwrap_or_else(|| {
            vec![Line::from("Failed to retrieve logs").style(Style::default().fg(Color::Red))]
        });

        Paragraph::new(log_text).block(block).render(area, buf);
    }

    /// Render the popup menu.
    fn render_popup(&self, area: Rect, buf: &mut Buffer) {
        let Some(popup) = &self.popup else {
            return;
        };

        // Get title styling based on context (character if applicable)
        let title = format!(" {} ", popup.title);
        let mut title_style = Style::default().add_modifier(Modifier::BOLD);

        // If context is a character index, color the title with their class color
        if let Some(char_idx) = popup.context_id
            && let Some(character) = self.characters.get(char_idx)
        {
            title_style = title_style.fg(character.character.class.class_colour());
        }

        let block = Block::bordered()
            .title(Line::styled(title, title_style))
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        let items: Vec<ListItem> = popup
            .items
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let hovered = i == popup.selected_index;
                let mut style = Style::default();
                if hovered {
                    style = style
                        .add_modifier(Modifier::BOLD)
                        .bg(HOVER_BG)
                        .fg(Color::White);
                }
                let line = Line::from(format!("{}{text}", highlight_symbol(hovered))).centered();
                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(block);

        Widget::render(Clear, area, buf);
        Widget::render(list, area, buf);
    }

    /// Render the top title bar.
    fn top_bar(&self, area: Rect, buf: &mut Buffer) {
        let branch_display = self.get_selected_branch_install().map_or_else(
            || "No branch selected".to_string(),
            wow::WowInstall::display_branch_name,
        );
        let title_text = format!(" ChronoBind - {branch_display} ");
        let line_style = Style::default().fg(Color::White);
        let title_line = Line::from(Span::styled(title_text, Style::default())).style(line_style);
        title_line.render(area, buf);
    }

    /// Render the bottom status bar.
    #[allow(clippy::cast_possible_truncation)]
    fn bottom_bar(&self, area: Rect, buf: &mut Buffer) {
        let suffix_options = ["q: Quit".to_string()];
        let status_elements = if self.show_console {
            vec!["↑/↓: Scroll", "PgUp/PgDn: Fast Scroll", "Home/End: Jump"]
        } else {
            match self.input_mode {
                InputMode::Navigation => {
                    let mut items = vec!["↑/↓: Nav", "↵/→/Space: Select", "b: Backup", "c: Copy"];
                    if self.copied_char.is_some() {
                        items.push("v: Paste");
                    }
                    items
                }
                InputMode::FileSelection => vec![
                    "↑/↓: Nav",
                    "←: Back",
                    "Space/↵/→: Toggle",
                    "Ctrl+A: Select All",
                    "b: Backup",
                    "c: Copy",
                ],
                InputMode::Popup => vec!["↑/↓: Nav", "↵/Space: Select", "Esc: Close"],
            }
        };

        let line_style = Style::default().bg(Color::White).fg(Color::Black);

        let final_text = status_elements
            .iter()
            .map(std::string::ToString::to_string)
            .chain(suffix_options)
            .join(" | ");

        let right_line = if let Some(char_idx) = &self.copied_char
            && let Some(copied_char) = self.characters.get(*char_idx)
        {
            let char_text = copied_char.display_name(true);
            Line::from(vec![
                Span::from(" Copying: "),
                Span::from(char_text)
                    .style(Style::default().fg(copied_char.character.class.class_colour())),
            ])
            .style(Style::default().bg(Color::Black).fg(Color::White))
        } else {
            Line::from("")
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),                            // left text takes remaining space
                Constraint::Length(right_line.width() as u16), // right text fixed width
            ])
            .split(area);

        let status_line = Line::from(Span::styled(final_text, Style::default())).style(line_style);

        status_line.render(chunks[0], buf);
        right_line.render(chunks[1], buf);
    }
}

// Popups..
impl ChronoBindApp {
    /// Show the backup options popup for the given character index.
    pub fn show_backup_popup(&mut self, char_idx: usize) {
        if self.characters.get(char_idx).is_none() {
            log::error!("Invalid character index for backup popup: {char_idx}");
            return;
        }
        let popup = PopupState::new(
            " Backup Options ",
            vec![
                "Backup selected files".to_string(),
                "Backup all files".to_string(),
                "Restore from backup".to_string(),
            ],
            Box::new(handle_character_backup_popup),
        )
        .with_context(char_idx);
        self.open_popup(popup);
    }

    /// Show the restore from backup popup for the given character index.
    pub fn show_restore_popup(&mut self, char_idx: usize) {
        self.refresh_character_backups(char_idx);
        let Some(character) = self.characters.get(char_idx) else {
            log::error!("Invalid character index for restore popup: {char_idx}");
            return;
        };
        let items = character
            .character
            .backups
            .iter()
            .map(|backup| {
                format!(
                    "{} {}{}",
                    backup.char_name,
                    display_backup_time(&backup.timestamp),
                    if backup.is_paste { " (Pasted)" } else { "" }
                )
            })
            .collect_vec();
        let popup = PopupState::new(
            format!(" Restore {} ", self.characters[char_idx].display_name(true)),
            items,
            Box::new(handle_character_restore_popup),
        )
        .with_context(char_idx);
        self.open_popup(popup);
    }

    /// Show the paste confirmation popup for the given character index and file count.
    pub fn show_paste_confirm_popup(&mut self, char_idx: usize, file_count: usize) {
        let title = " Are you sure? ";
        let plural = if file_count == 1 { "" } else { "s" };
        let Some(character) = self.characters.get(char_idx) else {
            log::error!("Invalid character index for paste confirm popup: {char_idx}");
            return;
        };
        let items = vec![
            format!(
                "Paste {} file{} to {}",
                file_count,
                plural,
                character.display_name(true)
            ),
            "Cancel".to_string(),
        ];
        let popup = PopupState::new(title, items, Box::new(handle_paste_confirm_popup))
            .with_context(char_idx);
        self.open_popup(popup);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
/// with minimum width and height constraints
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    const MIN_WIDTH: u16 = 35;
    const MIN_HEIGHT: u16 = 10;

    let width = ((area.width * percent_x) / 100).max(MIN_WIDTH);
    let height = ((area.height * percent_y) / 100).max(MIN_HEIGHT);

    let vertical = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

impl<'a> From<(&'a Character, &'a wow::WowInstall)> for backend::CharacterWithInstall<'a> {
    fn from(val: (&'a Character, &'a wow::WowInstall)) -> Self {
        backend::CharacterWithInstall {
            character: &val.0.character,
            install: val.1,
        }
    }
}

/// Handle the character backup popup selection once an option is chosen.
fn handle_character_backup_popup(
    app: &mut ChronoBindApp,
    selected_index: usize,
    option: &str,
    char_idx: usize,
) -> bool {
    let Some(character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for backup popup: {char_idx}");
        return true;
    };

    log::info!(
        "Selected backup option: {option} for character {} on branch {}",
        character.0.name(),
        character.1.branch_ident
    );

    if selected_index == 2 {
        app.show_restore_popup(char_idx);
        return false;
    }

    let backup_result = if selected_index == 0 {
        let selected_files = character.0.get_all_selected_files();
        backend::backup_character_files(&character.into(), &selected_files, false)
    } else if selected_index == 1 {
        backend::backup_character(&character.into(), false)
    } else {
        log::error!("Invalid backup option selected: {selected_index}");
        return true; // Close popup on error
    };

    match backup_result {
        Ok(backup_path) => {
            log::info!(
                "Backup completed successfully for character {}. Backup file created at: {}",
                character.0.name(),
                backup_path.display()
            );
        }
        Err(e) => {
            log::error!("Backup failed for character {}: {}", character.0.name(), e);
        }
    }

    true
}

/// Handle the character restore popup selection once an option is chosen.
fn handle_character_restore_popup(
    app: &mut ChronoBindApp,
    selected_index: usize,
    option: &str,
    char_idx: usize,
) -> bool {
    let Some(character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for backup popup: {char_idx}");
        return true;
    };

    log::info!(
        "Selected restore option: {option} for character {} on branch {}",
        character.0.name(),
        character.1.branch_ident
    );

    let Some(backup) = character.0.character.backups.get(selected_index).cloned() else {
        log::error!(
            "Invalid backup selection index: {selected_index} for character {}",
            character.0.name()
        );
        return true;
    };

    match backend::restore_backup(&character.into(), &backup.path, app.config.mock_mode) {
        Ok(files_restored) => {
            log::info!(
                "Restore completed successfully for character {}. {} files restored from backup '{}'.",
                character.0.name(),
                files_restored,
                backup.formatted_timestamp()
            );
        }
        Err(e) => {
            log::error!(
                "Restore failed for character {} from backup '{}': {}",
                character.0.name(),
                backup.formatted_timestamp(),
                e
            );
        }
    }

    true
}

/// Handle the paste confirmation popup selection once an option is chosen.
fn handle_paste_confirm_popup(
    app: &mut ChronoBindApp,
    selected_index: usize,
    _option: &str,
    char_idx: usize,
) -> bool {
    let Some(dest_character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for paste popup: {char_idx}");
        return true;
    };
    let Some(source_char_idx) = &app.copied_char else {
        log::error!("No character found for paste operation!");
        return true;
    };
    let Some(source_character) = app.character_with_install(*source_char_idx) else {
        log::error!("Invalid source character index for paste operation: {source_char_idx}");
        return true;
    };

    if selected_index == 0 {
        let files_to_paste = source_character.0.get_all_selected_files();

        match backend::paste_character_files(
            &dest_character.into(),
            &source_character.into(),
            &files_to_paste,
            app.config.mock_mode,
        ) {
            Ok(pasted_files) => {
                if pasted_files == files_to_paste.len() {
                    log::info!(
                        "Paste operation completed successfully for character {}. {} files pasted.",
                        dest_character.0.name(),
                        pasted_files
                    );
                } else {
                    log::warn!(
                        "Paste operation completed with partial success for character {}. {}/{} files pasted.",
                        dest_character.0.name(),
                        pasted_files,
                        files_to_paste.len()
                    );
                }
                app.refresh_character(char_idx);
            }
            Err(e) => {
                log::error!(
                    "Paste operation failed for character {} from {}: {}",
                    dest_character.0.name(),
                    source_character.0.name(),
                    e
                );
            }
        }
    }

    true
}

/// Format a `DateTime<Local>` for display in the UI.
#[must_use]
pub fn display_backup_time(dt: &chrono::DateTime<chrono::Local>) -> String {
    dt.format(backend::DISPLAY_TIME_FORMAT).to_string()
}
