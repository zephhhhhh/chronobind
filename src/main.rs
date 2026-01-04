#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod backend;
pub mod config;
pub mod files;
pub mod palette;
pub mod popups;
pub mod tui_log;
pub mod ui;
pub mod widgets;
pub mod wow;

use ratatui::widgets::Widget;
use widgets::character_list::{CharacterListWidget, NavigationAction};
use widgets::console::ConsoleWidget;
use widgets::file_list::{FileListConfig, FileListWidget, FileSelectionAction};

use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::Context;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::{DefaultTerminal, Frame};

use crate::config::ChronoBindAppConfig;
use crate::palette::{STD_BG, STD_FG};
use crate::popups::backup_manager_popup::{BackupManagerPopup, BackupManagerPopupCommand};
use crate::popups::backup_popup::{BackupPopup, BackupPopupCommand};
use crate::popups::branch_popup::{BranchPopup, BranchPopupCommand};
use crate::popups::confirm_popup::{CONFIRM_POPUP_ID, ConfirmationPopup};
use crate::popups::options_popup::{OptionsPopup, OptionsPopupCommand};
use crate::popups::restore_popup::{RestorePopup, RestorePopupCommand};
use crate::ui::messages::{AppMessage, ConfirmActionText, PopupMessage};
use crate::ui::{Character, CharacterWithIndex, CharacterWithInstall};
use crate::widgets::popup::{Popup, PopupPtr};
use crate::wow::WowBackup;

/// Entry point..
fn main() -> Result<()> {
    // Bootstrap better panic handling..
    color_eyre::install()?;

    // Initialize logging that binds to our TUI..
    tui_log::init_tui_logger(if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });

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

/// Different input modes for the application.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum InputMode {
    /// Navigating the character list.
    #[default]
    Navigation,
    /// Selecting files for a character.
    FileSelection,
    /// Interacting with a custom popup.
    Popup,
}

/// Main application.
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

    /// Console output widget.
    console_widget: ConsoleWidget,

    /// Character list widget for displaying characters.
    character_list_widget: CharacterListWidget,
    /// File list widget for displaying character files.
    file_list_widget: FileListWidget,

    /// The previous input mode before opening a popup.
    popup_previous_input: Option<InputMode>,
    /// Current popup, if any.
    popup: Option<PopupPtr>,

    /// Confirmation popup, if any.
    confirm_popup: Option<ConfirmationPopup>,
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

        let config = match ChronoBindAppConfig::load_config_or_default() {
            Ok(cfg) => cfg,
            Err(e) => {
                log::error!("Failed to load configuration file: {e}");
                ChronoBindAppConfig::default()
            }
        };

        let mut app = Self {
            config,
            should_exit: false,

            selected_branch: None,
            wow_installations: wow_installs,
            characters: Vec::new(),
            copied_char: None,

            input_mode: InputMode::Navigation,

            console_widget: ConsoleWidget::new(),
            character_list_widget: CharacterListWidget::new(),
            file_list_widget: FileListWidget::new(),

            popup_previous_input: None,
            popup: None,
            confirm_popup: None,
        };

        let branch_to_load = app
            .config
            .preferred_branch
            .clone()
            .unwrap_or_else(|| wow::WOW_RETAIL_IDENT.to_string());
        app.set_selected_branch(&branch_to_load);

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

    /// Get the character with its associated index for use in popups.
    #[inline]
    #[must_use]
    pub fn character_with_index(&self, index: usize) -> Option<CharacterWithIndex> {
        let character = self.characters.get(index)?;
        Some(CharacterWithIndex(character.clone(), index))
    }

    /// Get the character with its associated install for the character at the given index.
    #[inline]
    #[must_use]
    pub fn character_with_install(&self, index: usize) -> Option<CharacterWithInstall<'_>> {
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

    /// Set the currently selected branch identifier, and load the appropriate characters.
    pub fn set_selected_branch(&mut self, branch: &str) -> bool {
        let Some(install) = self.find_wow_branch(branch).cloned() else {
            return false;
        };
        self.character_list_widget.branch_display = Some(install.display_branch_name());
        self.selected_branch = Some(branch.to_string());
        let Some(characters) = self.load_branch_characters(branch) else {
            return false;
        };
        self.characters = characters;
        true
    }

    /// Load the characters from a given `WoW` branch identifier.
    pub fn load_branch_characters(&mut self, branch: &str) -> Option<Vec<Character>> {
        self.characters.clear();
        self.character_list_widget.state.select(Some(0));
        self.copied_char = None;

        let Some(install) = self.find_wow_branch(branch) else {
            log::error!("No WoW installation found for branch: {branch}");
            return None;
        };

        let Some(characters) = install
            .find_all_characters_and_files()
            .map(|chars| chars.iter().map(Character::new).collect::<Vec<_>>())
        else {
            log::error!(
                "Failed to find characters in installation at {}",
                install.install_path
            );
            return None;
        };

        Some(characters)
    }
}

impl ChronoBindApp {
    /// Runs the main application loop.
    /// # Errors
    /// Returns an error if event polling or reading fails.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        match wow::locate_wow_installs() {
            Ok(installs) => {
                log::debug!("Located {} WoW installations:", installs.len());
                for install in installs {
                    log::debug!(
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
                        log::debug!(
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

        if let Some(popup) = &mut self.popup {
            popup.render(frame);
        }
        if let Some(confirm_popup) = &mut self.confirm_popup {
            confirm_popup.render(frame);
        }
    }

    /// Handle key down events.
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char('r' | 'R') => {
                log::debug!("Refreshing character list..");
                if let Some(branch) = self.selected_branch.clone() {
                    self.set_selected_branch(&branch);
                } else {
                    log::warn!("No branch selected to refresh characters");
                }
                log::debug!("Character list refreshed.");
            }
            KeyCode::Char('`' | '¬' | '~') => {
                self.console_widget.toggle_show();
            }
            KeyCode::Char('t' | 'T') => {
                self.show_branch_select_popup();
            }
            KeyCode::Char('o' | 'O') => {
                self.show_options_popup();
            }
            KeyCode::Char('q' | 'Q') => {
                log::debug!("Quit requested");
                self.should_exit = true;
            }
            _ => {}
        }

        if self.console_widget.is_visible() {
            self.console_widget.handle_input(key);
        } else {
            match self.input_mode {
                InputMode::Navigation => self.handle_char_navigation_commands(key),
                InputMode::FileSelection => self.handle_file_selection_commands(key),
                InputMode::Popup => {}
            }
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
                if let Some(character) = self.characters.get(char_idx) {
                    self.copied_char = Some(char_idx);
                    let copied_files = character.total_selected_count();
                    log::info!(
                        "Copied {copied_files} files from {}",
                        character.display_name(true)
                    );
                }
            }
            NavigationAction::Paste(target_char_idx) => {
                if let Some(source_char_idx) = self.copied_char {
                    if source_char_idx == target_char_idx {
                        log::warn!(
                            "Cannot paste files onto the same character they were copied from"
                        );
                    } else {
                        let Some(dest_char) = self.characters.get(target_char_idx) else {
                            log::error!("Failed to get target character for paste operation!");
                            return;
                        };
                        let files_to_paste = self.characters[source_char_idx]
                            .get_all_selected_files()
                            .len();
                        let plural = if files_to_paste == 1 { "" } else { "s" };
                        let prompt = Span::from(format!("Paste {files_to_paste} file{plural} to "));
                        let char_name = dest_char.display_span(true).bold();
                        self.handle_popup_message(
                            &AppMessage::Paste(target_char_idx)
                                .with_confirm_and_line(Line::from(vec![prompt, char_name])),
                        );
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

    /// Handle a single popup command.
    fn handle_popup_message(&mut self, command: &AppMessage) {
        match command {
            AppMessage::Backup(char_idx, backup_command) => match backup_command {
                BackupPopupCommand::ManageBackups => {
                    self.show_manage_backups_popup(*char_idx, 0);
                }
                BackupPopupCommand::BackupSelectedFiles => {
                    perform_character_backup(self, *char_idx, true);
                }
                BackupPopupCommand::BackupAllFiles => {
                    perform_character_backup(self, *char_idx, false);
                }
                BackupPopupCommand::RestoreFromBackup => {
                    if !self.refresh_character_backups(*char_idx) {
                        log::error!("Failed to refresh backups before opening restore menu!");
                        return;
                    }
                    let Some(character) = self.character_with_index(*char_idx) else {
                        log::error!("Failed to get character for restore popup!");
                        return;
                    };
                    self.open_popup(RestorePopup::new(character, None));
                }
                BackupPopupCommand::RestoreFromCopiedBackups => {
                    let Some(source_char_idx) = self.copied_char else {
                        log::error!("No character found for restore operation!");
                        return;
                    };
                    if !self.refresh_character_backups(source_char_idx) {
                        log::error!("Failed to refresh backups before opening restore menu!");
                        return;
                    }
                    let Some(dest_char) = self.character_with_index(*char_idx) else {
                        log::error!("Failed to get character for restore popup!");
                        return;
                    };
                    let Some(source_char) = self.character_with_index(source_char_idx) else {
                        log::error!("Failed to get source character for restore popup!");
                        return;
                    };
                    self.open_popup(RestorePopup::new(dest_char, Some(source_char)));
                }
            },
            AppMessage::Restore(char_idx, restore_command) => match restore_command {
                RestorePopupCommand::RestoreBackup(backup_index) => {
                    perform_character_restore(self, *backup_index, *char_idx);
                }
            },
            AppMessage::Paste(char_idx) => {
                let Some(source_char_idx) = &self.copied_char else {
                    log::error!("No character found for paste operation!");
                    return;
                };
                if perform_character_paste(self, *source_char_idx, *char_idx) {
                    manage_character_backups(self, *char_idx);
                }
            }
            AppMessage::Branch(BranchPopupCommand::SelectBranch(chosen_branch)) => {
                log::info!("Switching to branch: {chosen_branch}");
                self.set_selected_branch(chosen_branch);
            }
            AppMessage::Options(OptionsPopupCommand::UpdateConfiguration(new_config)) => {
                log::debug!("Updating application configuration.");
                self.config = new_config.clone();
                self.config.save_to_file().unwrap_or_else(|e| {
                    log::error!("Failed to save configuration file: {e}");
                });
            }
            AppMessage::BackupManager(char_idx, cmd) => {
                match cmd {
                    BackupManagerPopupCommand::DeleteBackup(backup_index) => {
                        perform_backup_deletion(self, *char_idx, *backup_index);
                    }
                    BackupManagerPopupCommand::ToggleBackupPin(backup_index) => {
                        perform_backup_pin_toggle(self, *char_idx, *backup_index);
                    }
                }
                self.refresh_character_backups(*char_idx);
                if let Some(character) = self.character_with_index(*char_idx) {
                    self.send_popup_message(&PopupMessage::UpdateCharacter(character));
                }
            }
            AppMessage::ConfirmAction(action, action_line) => {
                log::debug!("Showing confirmation popup for action.");
                self.show_confirmation_popup(*action.clone(), action_line.clone());
            }
        }
    }

    /// Dispatch events to the popup, if any, also handlings closing the popup.
    /// Returns true if the event was handled and should not propagate further.
    fn dispatch_popup_commands(&mut self, ev: &Event) -> bool {
        let Some(popup) = self.active_popup_mut() else {
            return false;
        };
        let popup_id = popup.popup_identifier();
        let blocking = popup.handle_event(ev);
        let closing = popup.should_close();
        if let Some(commands) = popup.commands() {
            for command in commands {
                self.handle_popup_message(&command);
            }
        }
        if closing {
            if popup_id == CONFIRM_POPUP_ID {
                self.close_confirmation_popup();
            } else {
                self.close_popup();
            }
        }
        blocking
    }

    /// Handle input events.
    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(250)).context("Event poll failed")? {
            let ev = event::read().context("Event read failed")?;

            if !self.dispatch_popup_commands(&ev)
                && let Event::Key(k) = ev
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
        if self.console_widget.is_visible() {
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
        self.console_widget.render(area, buf);
    }

    /// Render the top title bar.
    #[allow(clippy::cast_possible_truncation)]
    fn top_bar(&self, area: Rect, buf: &mut Buffer) {
        let mock = if self.config.mock_mode {
            "[Safe] "
        } else {
            Default::default()
        };
        let line_style = Style::default().fg(STD_FG);
        let title_span = Span::styled(format!(" ChronoBind {mock}"), line_style);

        let copy_display = if let Some(char_idx) = &self.copied_char
            && let Some(copied_char) = self.characters.get(*char_idx)
        {
            Line::from(vec![
                Span::from(" Copying: "),
                copied_char.display_span(true),
                Span::from(format!(" ({})", copied_char.total_selected_count())),
            ])
            .bg(STD_BG)
            .fg(STD_FG)
        } else {
            Line::from("")
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),                              // left text takes remaining space
                Constraint::Length(copy_display.width() as u16), // right text fixed width
            ])
            .split(area);

        title_span.render(chunks[0], buf);
        copy_display.render(chunks[1], buf);
    }

    /// Render the bottom status bar.
    fn bottom_bar(&self, area: Rect, buf: &mut Buffer) {
        const BOTTOM_BAR_SEP: &str = " | ";

        let suffix_options = [
            "T: WoW Version".to_string(),
            "(O)ptions".to_string(),
            "(Q)uit".to_string(),
        ];
        let status_elements = if self.console_widget.is_visible() {
            vec!["↑/↓", "PgUp/PgDn: Fast Scroll", "Home/End: Jump"]
        } else {
            match self.input_mode {
                InputMode::Navigation => {
                    let mut items = vec!["↑/↓", "↵/→/Space: Select", "(B)ackup", "(C)opy"];
                    if self.copied_char.is_some() {
                        items.push("V: Paste");
                    }
                    items
                }
                InputMode::FileSelection => vec![
                    "↑/↓",
                    "←: Back",
                    "Space/↵/→: Toggle",
                    "Ctrl+A: Select All",
                    "(B)ackup",
                    "(C)opy",
                ],
                InputMode::Popup => self.active_popup().map_or_else(Vec::new, |popup| {
                    popup.bottom_bar_options().unwrap_or_default()
                }),
            }
        };

        let line_style = Style::default().bg(STD_FG).fg(STD_BG);

        let final_text = if self.input_mode == InputMode::Popup || self.console_widget.is_visible()
        {
            status_elements
                .iter()
                .map(std::string::ToString::to_string)
                .join(BOTTOM_BAR_SEP)
        } else {
            status_elements
                .iter()
                .map(std::string::ToString::to_string)
                .chain(suffix_options)
                .join(BOTTOM_BAR_SEP)
        };

        let status_line = Line::styled(format!(" {final_text}"), line_style);
        status_line.render(area, buf);
    }
}

// Popups..
impl ChronoBindApp {
    /// Get a mutable reference to the active popup, or confirmation popup if any.
    #[inline]
    #[must_use]
    pub fn active_popup_mut(&mut self) -> Option<&mut (dyn Popup + Send + Sync)> {
        if let Some(popup) = &mut self.confirm_popup {
            Some(popup)
        } else if let Some(popup) = &mut self.popup {
            Some(popup.as_mut())
        } else {
            None
        }
    }

    /// Get a reference to the active popup, or confirmation popup if any.
    #[inline]
    #[must_use]
    pub fn active_popup(&self) -> Option<&(dyn Popup + Send + Sync)> {
        self.confirm_popup.as_ref().map_or_else(
            || self.popup.as_ref().map(std::convert::AsRef::as_ref),
            |popup| Some(popup),
        )
    }

    /// Set the input mode to popup, saving the previous mode, if not already set.
    const fn set_popup_input(&mut self) {
        if self.popup_previous_input.is_none() {
            self.popup_previous_input = Some(self.input_mode);
        }
        self.input_mode = InputMode::Popup;
    }

    /// Restore the input mode from before the popup was opened.
    fn restore_input_from_popup(&mut self) {
        self.input_mode = self.popup_previous_input.take().unwrap_or_default();
    }

    /// Open a popup menu with the given state.
    pub fn open_popup<T: Popup + Send + Sync + 'static>(&mut self, popup: T) {
        self.popup = Some(Box::new(popup));
        self.set_popup_input();
    }

    /// Close the current popup, restoring previous input mode.
    pub fn close_popup(&mut self) {
        let (popup_id, should_close) = if let Some(popup) = &self.popup {
            (popup.popup_identifier().to_string(), popup.should_close())
        } else {
            log::warn!("No popup to close");
            return;
        };
        log::debug!("Closing popup: {popup_id} (should_close: {should_close})");
        self.restore_input_from_popup();
        self.popup = None;
    }

    /// Send a message to the current popup.
    #[inline]
    pub fn send_popup_message(&mut self, message: &PopupMessage) {
        if let Some(popup) = &mut self.popup {
            popup.process_message(message);
        }
    }

    /// Show a generic confirmation popup for the given action.
    pub fn show_confirmation_popup(
        &mut self,
        action: AppMessage,
        action_line: Option<ConfirmActionText>,
    ) {
        self.confirm_popup = Some(ConfirmationPopup::new(action, action_line));
    }

    /// Close the confirmation popup.
    pub fn close_confirmation_popup(&mut self) {
        self.confirm_popup = None;
        if self.popup.is_none() {
            self.restore_input_from_popup();
        }
    }

    /// Show the backup options popup for the given character index.
    pub fn show_backup_popup(&mut self, char_idx: usize) {
        let Some(character) = self.character_with_index(char_idx) else {
            log::error!("Invalid character index for backup popup: {char_idx}");
            return;
        };

        let copied_char = self
            .copied_char
            .and_then(|idx| self.character_with_index(idx));

        self.open_popup(BackupPopup::new(character, copied_char));
    }

    /// Show the backup manager popup for the given character index, and selected backup index.
    pub fn show_manage_backups_popup(&mut self, char_idx: usize, selected_index: usize) {
        let Some(character) = self.character_with_index(char_idx) else {
            log::error!("Invalid character index for backup manager popup: {char_idx}");
            return;
        };

        self.open_popup(BackupManagerPopup::new(character, selected_index));
    }

    /// Show the branch selection popup.
    pub fn show_branch_select_popup(&mut self) {
        self.open_popup(BranchPopup::new(
            self.wow_installations.clone(),
            self.selected_branch.clone(),
        ));
    }

    /// Show the options popup.
    pub fn show_options_popup(&mut self) {
        self.open_popup(OptionsPopup::new(
            self.config.clone(),
            self.wow_installations.clone(),
        ));
    }
}

impl<'a> From<(&'a Character, &'a wow::WowInstall)> for backend::CharacterWithInstall<'a> {
    fn from(val: (&'a Character, &'a wow::WowInstall)) -> Self {
        backend::CharacterWithInstall {
            character: &val.0.character,
            install: val.1,
        }
    }
}

/// Perform the character backup operation.
fn perform_character_backup(app: &ChronoBindApp, char_idx: usize, selective: bool) -> bool {
    let Some(character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for backup popup: {char_idx}");
        return true;
    };

    log::info!(
        "Backing up {} for character {} on branch {}",
        if selective {
            "selected files"
        } else {
            "all files"
        },
        character.0.name(),
        character.1.branch_ident
    );

    let backup_result = if selective {
        let selected_files = character.0.get_all_selected_files();
        backend::backup_character_files(&character.into(), &selected_files, false, false)
    } else {
        backend::backup_character(&character.into(), false, false)
    };

    match backup_result {
        Ok(backup_path) => {
            log::info!(
                "Backup completed successfully for character {}. Backup file created at: {}",
                character.0.name(),
                backup_path.display()
            );
            true
        }
        Err(e) => {
            log::error!("Backup failed for character {}: {}", character.0.name(), e);
            false
        }
    }
}

/// Perform the character restore operation.
fn perform_character_restore(app: &ChronoBindApp, backup_index: usize, char_idx: usize) -> bool {
    perform_character_restore_from(app, char_idx, char_idx, backup_index)
}

/// Perform the character restore operation, from a source character to a destination character.
fn perform_character_restore_from(
    app: &ChronoBindApp,
    dest_char_index: usize,
    src_char_idx: usize,
    backup_index: usize,
) -> bool {
    let Some(dest_char) = app.character_with_install(dest_char_index) else {
        log::error!("Invalid destination character index for backup popup: {dest_char_index}");
        return true;
    };
    let Some(src_char) = app.character_with_install(src_char_idx) else {
        log::error!("Invalid source character index for backup popup: {src_char_idx}");
        return true;
    };
    let Some(backup) = src_char.0.backups().get(backup_index).cloned() else {
        log::error!(
            "Invalid backup selection index: {backup_index} for character {}",
            src_char.0.name()
        );
        return true;
    };

    restore_from_backup(dest_char, &backup, app.config.mock_mode)
}

/// Perform the character restore operation.
fn restore_from_backup(dest_char: CharacterWithInstall, backup: &WowBackup, mock: bool) -> bool {
    log::info!(
        "Restoring backup `{}` for character {} on branch {}",
        backup.formatted_name(),
        dest_char.0.name(),
        dest_char.1.branch_ident
    );

    match backend::restore_backup(&dest_char.into(), &backup.path, mock) {
        Ok(files_restored) => {
            log::info!(
                "Restore completed successfully for character {}. {} files restored from backup '{}'.",
                dest_char.0.name(),
                files_restored,
                backup.formatted_name()
            );
            true
        }
        Err(e) => {
            log::error!(
                "Restore failed for character {} from backup '{}': {}",
                dest_char.0.name(),
                backup.formatted_name(),
                e
            );
            false
        }
    }
}

/// Perform the character paste operation.
fn perform_character_paste(
    app: &ChronoBindApp,
    source_char_idx: usize,
    dest_char_idx: usize,
) -> bool {
    let Some(dest_character) = app.character_with_install(dest_char_idx) else {
        log::error!("Invalid character index for paste popup: {dest_char_idx}");
        return false;
    };
    let Some(source_character) = app.character_with_install(source_char_idx) else {
        log::error!("Invalid source character index for paste operation: {source_char_idx}");
        return false;
    };

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
            true
        }
        Err(e) => {
            log::error!(
                "Paste operation failed for character {} from {}: {}",
                dest_character.0.name(),
                source_character.0.name(),
                e
            );
            false
        }
    }
}

/// Perform the backup pin toggle operation.
fn perform_backup_pin_toggle(app: &ChronoBindApp, char_idx: usize, backup_index: usize) -> bool {
    let Some(character) = app.characters.get(char_idx) else {
        log::error!("Invalid character index for backup pin toggle: {char_idx}");
        return false;
    };
    let Some(backup) = character.backups().get(backup_index).cloned() else {
        log::error!(
            "Invalid backup selection index: {backup_index} for character {}",
            character.name()
        );
        return false;
    };

    log::info!(
        "Toggling pin state for backup `{}` of character {}",
        backup.formatted_name(),
        character.name()
    );

    match backend::toggle_backup_pin(&backup, app.config.mock_mode) {
        Ok(()) => {
            log::info!(
                "Backup pin state toggled successfully for backup `{}` of character {}",
                backup.formatted_name(),
                character.name()
            );
            true
        }
        Err(e) => {
            log::error!(
                "Failed to toggle pin state for backup `{}` of character {}: {}",
                backup.formatted_name(),
                character.name(),
                e
            );
            false
        }
    }
}

fn perform_backup_deletion(app: &ChronoBindApp, char_idx: usize, backup_index: usize) -> bool {
    let Some(character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for backup deletion: {char_idx}");
        return false;
    };
    let Some(backup) = character.0.backups().get(backup_index).cloned() else {
        log::error!(
            "Invalid backup selection index: {backup_index} for character {}",
            character.0.name()
        );
        return false;
    };

    match backend::delete_backup_file(&backup, false, app.config.mock_mode) {
        Ok(deleted) => {
            if deleted {
                log::info!(
                    "Backup `{}` deleted successfully for character {}",
                    backup.formatted_name(),
                    character.0.name()
                );
            } else {
                log::info!(
                    "Backup `{}` was pinned and not deleted for character {}",
                    backup.formatted_name(),
                    character.0.name()
                );
            }
            true
        }
        Err(e) => {
            log::error!(
                "Failed to delete backup `{}` for character {}: {}",
                backup.formatted_name(),
                character.0.name(),
                e
            );
            false
        }
    }
}

/// Manage automatic backups for the given character after an operation.
fn manage_character_backups(app: &mut ChronoBindApp, char_idx: usize) -> bool {
    app.refresh_character(char_idx);

    let Some(character) = app.character_with_install(char_idx) else {
        log::error!("Invalid character index for backup management: {char_idx}");
        return true;
    };

    let Some(max_backups) = app.config.maximum_auto_backups else {
        log::debug!("Automatic backup management is disabled.");
        return true;
    };

    log::debug!(
        "Managing automatic backups for character {} with max {} backups.",
        character.0.name(),
        max_backups
    );

    match backend::manage_character_backups(&character.into(), max_backups, app.config.mock_mode) {
        Ok(removed_count) => {
            if removed_count > 0 {
                log::info!(
                    "Automatic backup management completed for character {}. {} old backups removed.",
                    character.0.name(),
                    removed_count
                );
            }
            true
        }
        Err(e) => {
            log::error!(
                "Automatic backup management failed for character {}: {}",
                character.0.name(),
                e
            );
            false
        }
    }
}
