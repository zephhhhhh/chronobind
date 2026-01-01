#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod backend;
pub mod palette;
pub mod tui_log;
pub mod widgets;
pub mod wow;

use widgets::popup::PopupState;

#[allow(clippy::wildcard_imports)]
use palette::*;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::Context;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListDirection, ListItem, ListState, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

use crate::wow::WowCharacter;

fn main() -> Result<()> {
    color_eyre::install()?;

    tui_log::init_tui_logger(log::LevelFilter::Debug);

    let mut app = ChronoBindApp::new();
    let mut terminal = ratatui::init();

    let result = app.run(&mut terminal);

    ratatui::restore();

    result
}

#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChronoBindAppConfig {
    pub show_realm: bool,
    pub show_output: bool,
    pub show_friendly_names: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum InputMode {
    #[default]
    Navigation,
    FileSelection,
    Popup,
}

#[derive(Debug, Default)]
pub struct ChronoBindApp {
    should_exit: bool,
    #[allow(dead_code)]
    wow_installations: Vec<wow::WowInstall>,
    characters: Vec<Character>,
    input_mode: InputMode,
    config: ChronoBindAppConfig,
    debug_scroll_offset: usize,
    collapsed_realms: BTreeSet<String>,
    file_list_state: ListState,
    character_list_state: ListState,
    popup: Option<PopupState>,
    copied_files: Option<(usize, Vec<PathBuf>)>,
}

pub enum PopupKind {
    BackupPopup,
}

pub struct PopupInfo {
    pub kind: PopupKind,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
struct Character {
    pub character: WowCharacter,
    selected_config_files: Vec<bool>,
    selected_addon_files: Vec<bool>,
    addon_options_collapsed: bool,
}

impl Character {
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

    #[inline]
    #[must_use]
    pub fn config_files(&self) -> &[wow::WowCharacterFile] {
        &self.character.config_files
    }

    #[inline]
    #[must_use]
    pub fn addon_files(&self) -> &[wow::WowCharacterFile] {
        &self.character.addon_files
    }

    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_config_file_selected(&self, index: usize) -> bool {
        self.selected_config_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    #[inline]
    pub fn toggle_config_file_selected(&mut self, index: usize) -> bool {
        self.selected_config_files
            .get_mut(index)
            .is_some_and(|selected| {
                *selected = !*selected;
                *selected
            })
    }

    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_addon_file_selected(&self, index: usize) -> bool {
        self.selected_addon_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

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
    pub fn any_config_file_selected(&self) -> bool {
        self.selected_config_files.iter().any(|&s| s)
    }

    /// Check if any addon file is selected.
    #[inline]
    pub fn any_addon_file_selected(&self) -> bool {
        self.selected_addon_files.iter().any(|&s| s)
    }

    /// Check if any files (regular or addon) are selected.
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

#[derive(Debug, Clone, Copy)]
enum FileRowKind {
    File(usize),
    AddonHeader { collapsed: bool, count: usize },
    AddonFile(usize),
}

impl ChronoBindApp {
    #[inline]
    fn file_rows_for_character(character: &Character) -> Vec<FileRowKind> {
        let mut rows = Vec::new();

        for idx in 0..character.config_files().len() {
            rows.push(FileRowKind::File(idx));
        }

        let addon_count = character.addon_files().len();
        rows.push(FileRowKind::AddonHeader {
            collapsed: character.addon_options_collapsed,
            count: addon_count,
        });

        if !character.addon_options_collapsed {
            for idx in 0..addon_count {
                rows.push(FileRowKind::AddonFile(idx));
            }
        }

        rows
    }

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
            should_exit: false,
            wow_installations: wow_installs,
            characters: Vec::new(),
            debug_scroll_offset: 0,
            input_mode: InputMode::Navigation,
            collapsed_realms: BTreeSet::new(),
            popup: None,
            file_list_state: ListState::default(),
            character_list_state: ListState::default(),
            config: ChronoBindAppConfig {
                show_realm: false,
                show_output: false,
                show_friendly_names: true,
            },
            copied_files: None,
        };

        app.refresh_characters();

        app
    }

    pub fn refresh_characters(&mut self) {
        // Retail for now..
        let chars = self
            .wow_installations
            .iter()
            .find(|install| install.is_retail())
            .and_then(wow::WowInstall::find_all_characters_and_files)
            .map(|chars| chars.iter().map(Character::new).collect())
            .unwrap_or_default();

        self.characters = chars;
        self.character_list_state.select(Some(0));
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
        self.character_list_state.selected().unwrap_or(0)
    }

    /// Copy the selected files for the currently selected character.
    #[inline]
    #[must_use]
    pub fn copy_selected(&mut self) -> bool {
        if let Some(char_idx) = self.get_selected_character_index()
            && let Some(character) = self.characters.get(char_idx)
        {
            let selected_files = character.get_all_selected_files();
            if selected_files.is_empty() {
                return true;
            }
            self.copied_files = Some((char_idx, selected_files));
            true
        } else {
            false
        }
    }
}

impl ChronoBindApp {
    /// Open a popup menu with the given state.
    pub fn open_popup(&mut self, popup: PopupState) {
        self.popup = Some(popup);
        self.input_mode = InputMode::Popup;
    }

    /// Show the backup options popup for the given character index.
    pub fn show_backup_popup(&mut self, char_idx: usize) {
        if self.characters.get(char_idx).is_none() {
            log::error!("Invalid character index for backup popup: {char_idx}");
            return;
        }
        let popup = PopupState::new(
            "Backup Options",
            vec![
                "Backup Selected Files".to_string(),
                "Backup All Files".to_string(),
            ],
            Box::new(handle_character_backup_popup),
        )
        .with_context(char_idx);
        self.open_popup(popup);
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

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(frame.area());

        self.main_screen(chunks[0], frame.buffer_mut());
        self.bottom_bar(chunks[1], frame.buffer_mut());

        // Render popup on top if it's open
        if self.input_mode == InputMode::Popup {
            let popup_area = popup_area(frame.area(), 35, 30);
            self.render_popup(popup_area, frame.buffer_mut());
        }
    }

    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char('r') => {
                log::debug!("Refreshing character list..");
                self.refresh_characters();
                log::debug!("Character list refreshed.");
            }
            KeyCode::F(1) => {
                self.config.show_output = !self.config.show_output;
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

        if self.config.show_output {
            self.handle_console_output_keys(key);
        } else {
            match self.input_mode {
                InputMode::Navigation => self.handle_navigation_keys(key),
                InputMode::FileSelection => self.handle_file_selection_keys(key),
                InputMode::Popup => self.handle_popup_keys(key),
            }
        }
    }

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

    fn handle_navigation_keys(&mut self, key: &KeyEvent) {
        // Build the grouped structure to determine navigation
        let mut realms: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, character) in self.characters.iter().enumerate() {
            realms
                .entry(character.realm().to_string())
                .or_default()
                .push(i);
        }

        let mut abs_positions = Vec::new();
        let mut current_pos = 0;
        for (realm, char_indices) in &realms {
            abs_positions.push((current_pos, true, realm.clone()));
            current_pos += 1;

            // Only add characters if realm is not collapsed
            if !self.collapsed_realms.contains(realm) {
                for &char_idx in char_indices {
                    abs_positions.push((current_pos, false, format!("{char_idx}")));
                    current_pos += 1;
                }
            }
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('w') => {
                if let Some(selected) = self.character_list_state.selected() {
                    self.character_list_state
                        .select(Some(selected.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(selected) = self.character_list_state.selected() {
                    self.character_list_state
                        .select(Some(selected.saturating_add(1)));
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some((_, is_header, realm_or_idx)) = abs_positions.get(self.selected_index())
                {
                    if *is_header {
                        if self.collapsed_realms.contains(realm_or_idx) {
                            self.collapsed_realms.remove(realm_or_idx);
                        } else {
                            self.collapsed_realms.insert(realm_or_idx.clone());
                        }
                    } else {
                        // Character selected, enter file selection
                        self.input_mode = InputMode::FileSelection;
                        self.file_list_state.select(Some(0));
                        log::debug!("Entered file selection mode");
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Right => {
                if let Some((_, is_header, _)) = abs_positions.get(self.selected_index())
                    && !*is_header
                {
                    self.input_mode = InputMode::FileSelection;
                    self.file_list_state.select(Some(0));
                    log::debug!("Entered file selection mode");
                }
            }
            KeyCode::Char('b') => {
                if let Some((_, is_header, _)) = abs_positions.get(self.selected_index())
                    && !*is_header
                    && let Some(char_idx) = self.get_selected_character_index()
                {
                    self.show_backup_popup(char_idx);
                }
            }
            KeyCode::Char('c') => {
                if self.copy_selected() {
                    log::info!("Selected files copied to clipboard");
                } else {
                    log::error!("Failed to copy files from character!");
                }
            }
            _ => {}
        }
    }

    fn handle_file_selection_keys(&mut self, key: &KeyEvent) {
        let char_index = self.get_selected_character_index();
        let rows_meta = char_index
            .and_then(|idx| self.characters.get(idx))
            .map(Self::file_rows_for_character);
        let character = char_index.and_then(|idx| self.characters.get_mut(idx));

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char('a') if !ctrl => {
                self.input_mode = InputMode::Navigation;
            }
            KeyCode::Esc | KeyCode::Left => {
                self.input_mode = InputMode::Navigation;
            }
            KeyCode::Up | KeyCode::Char('w') => {
                if let Some(sel_index) = self.file_list_state.selected() {
                    self.file_list_state
                        .select(Some(sel_index.saturating_sub(1)));
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(sel_index) = self.file_list_state.selected() {
                    self.file_list_state.select(Some(sel_index + 1));
                }
            }
            KeyCode::Char(' ' | 'd') | KeyCode::Enter | KeyCode::Right => {
                let Some(selected_index) = self.file_list_state.selected() else {
                    return;
                };
                if let (Some(character), Some(rows)) = (character, rows_meta.as_ref())
                    && selected_index < rows.len()
                {
                    match rows[selected_index] {
                        FileRowKind::File(idx) => {
                            let selected = character.toggle_config_file_selected(idx);
                            let file_name = character.config_files()[idx].get_full_filename();
                            log::info!("File '{file_name}' toggled: {selected}");
                        }
                        FileRowKind::AddonHeader { .. } => {
                            if ctrl {
                                let selected = character.all_addon_files_selected();
                                character.set_all_addon_selected(!selected);
                                log::info!(
                                    "{} all addon files",
                                    if selected { "Deselected" } else { "Selected" }
                                );
                            } else {
                                character.addon_options_collapsed =
                                    !character.addon_options_collapsed;
                            }
                        }
                        FileRowKind::AddonFile(idx) => {
                            let selected = character.toggle_addon_file_selected(idx);
                            let file_name = character.addon_files()[idx].get_full_filename();
                            log::info!("Addon file '{file_name}' toggled: {selected}");
                        }
                    }
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(character) = character {
                    let all_selected = character.all_config_files_selected()
                        && character.all_addon_files_selected();
                    character.set_all_selected(!all_selected);
                    log::info!(
                        "All files {}",
                        if all_selected {
                            "deselected"
                        } else {
                            "selected"
                        }
                    );
                }
            }
            KeyCode::Char('b') => {
                if let Some(char_idx) = self.get_selected_character_index() {
                    self.show_backup_popup(char_idx);
                }
            }
            KeyCode::Char('c') => {
                if self.copy_selected() {
                    log::info!("Selected files copied to clipboard");
                } else {
                    log::error!("Failed to copy files from character!");
                }
            }
            _ => {}
        }
    }

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

        // Close popup after selection
        self.input_mode = InputMode::Navigation;
        self.popup = None;
    }

    fn on_event(&mut self, ev: &Event) {
        if let Event::Key(k) = ev
            && k.kind == KeyEventKind::Press
        {
            self.on_key_down(k);
        }
    }

    /// Get the actual character index from `selected_index`, accounting for grouped display
    fn get_selected_character_index(&self) -> Option<usize> {
        // Build the grouped structure
        let mut realms: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, character) in self.characters.iter().enumerate() {
            realms
                .entry(character.realm().to_string())
                .or_default()
                .push(i);
        }

        let mut current_pos = 0;
        for (realm, char_indices) in &realms {
            current_pos += 1;

            // Only process characters if realm is not collapsed
            if !self.collapsed_realms.contains(realm) {
                for &char_idx in char_indices {
                    if current_pos == self.selected_index() {
                        return Some(char_idx);
                    }
                    current_pos += 1;
                }
            }
        }
        None
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(250)).context("Event poll failed")? {
            let ev = event::read().context("Event read failed")?;
            self.on_event(&ev);
        }
        Ok(())
    }
}

// Ui..
impl ChronoBindApp {
    fn main_screen(&mut self, area: Rect, buf: &mut Buffer) {
        if self.config.show_output {
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

    fn character_list(&mut self, area: Rect, buf: &mut Buffer) {
        const INDENT_DEPTH: usize = 3;
        let indentation = " ".repeat(INDENT_DEPTH);

        let title = Line::styled(
            " Characters ",
            Style::default().add_modifier(Modifier::BOLD),
        );
        let block = Block::bordered().title(title).border_set(border::THICK);

        let mut realms: BTreeMap<String, Vec<(usize, &Character)>> = BTreeMap::new();
        for (i, character) in self.characters.iter().enumerate() {
            realms
                .entry(character.realm().to_string())
                .or_default()
                .push((i, character));
        }

        let mut items = Vec::new();

        for (realm, chars) in &realms {
            // Add realm header
            let is_collapsed = self.collapsed_realms.contains(realm);
            let hovered = self
                .character_list_state
                .selected()
                .is_some_and(|sel| sel == items.len());
            let collapse_icon = if is_collapsed { "▶" } else { "▼" };
            let header_style = Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(STD_FG)
                .add_modifier(Modifier::DIM);
            let content = format!(
                "{collapse_icon} {}[{realm}]",
                if hovered { "> " } else { "" }
            );
            items.push(ListItem::new(content).style(header_style));

            // Add characters in this realm (only if not collapsed)
            if !is_collapsed {
                for (_, character) in chars {
                    let hovered = self
                        .character_list_state
                        .selected()
                        .is_some_and(|sel| sel == items.len());
                    let style = Style::default();

                    let files_selected = character.any_file_selected();
                    let colour = character.character.class.class_colour();

                    let ui_span_text = format!("{indentation}{}", if hovered { "> " } else { "" });
                    let ui_span_source = if files_selected {
                        Span::from(format!("{ui_span_text}• ")).style(style.fg(SELECTED_FG))
                    } else {
                        Span::from(ui_span_text).style(style)
                    };

                    let main_span = Span::from(character.name()).style(style.fg(colour));
                    items.push(ListItem::new(Line::from(vec![ui_span_source, main_span])));
                }
            }
        }

        let list_view = List::new(items)
            .block(block)
            .style(Style::new().white())
            .highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
            .direction(ListDirection::TopToBottom);

        StatefulWidget::render(list_view, area, buf, &mut self.character_list_state);
    }

    fn file_row_file_item<'a, 'b: 'a>(
        &'a self,
        character: &Character,
        file_idx: usize,
        hovered: bool,
    ) -> ListItem<'b> {
        let file = &character.config_files()[file_idx];
        let selected = character.selected_config_files[file_idx];
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            SELECTED_FG
        } else if has_friendly && self.config.show_friendly_names {
            SPECIAL_FG
        } else {
            STD_FG
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui =
            Span::from(format!("[{}] \u{2699}  ", if selected { "✓" } else { " " })).style(style);

        if self.config.show_friendly_names && has_friendly {
            style = style.add_modifier(Modifier::ITALIC);
        }

        let file_name = file.display_name(self.config.show_friendly_names);
        let content = format!("{}{file_name}", if hovered { "> " } else { "" });

        ListItem::new(Line::from(vec![
            file_prefix_ui,
            Span::from(content).style(style),
        ]))
    }

    fn file_row_addon_header(
        character: &Character,
        count: usize,
        collapsed: bool,
        hovered: bool,
    ) -> ListItem<'_> {
        let any_addon_file_selected = character.any_addon_file_selected();
        let all_addon_file_selected = character.all_addon_files_selected();

        let colour = if all_addon_file_selected {
            SELECTED_FG
        } else if any_addon_file_selected {
            Color::Yellow
        } else {
            STD_FG
        };

        let icon = if collapsed { "▶" } else { "▼" };
        let label = format!("Addon Options ({count})");
        let content = format!(
            "{} {label}",
            if hovered {
                format!("{icon} >")
            } else {
                icon.to_string()
            }
        );

        let dropdown_style = Style::default()
            .fg(colour)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::ITALIC);

        ListItem::new(Line::from(content).style(dropdown_style))
    }

    fn file_row_addon_item<'a, 'b: 'a>(
        &'a self,
        character: &Character,
        file_idx: usize,
        hovered: bool,
    ) -> ListItem<'b> {
        const ADDON_IDENT: usize = 3;
        let indent = " ".repeat(ADDON_IDENT);

        let selected = character.selected_addon_files[file_idx];
        let file = &character.addon_files()[file_idx];
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            SELECTED_FG
        } else if has_friendly && self.config.show_friendly_names {
            SPECIAL_FG
        } else {
            STD_FG
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui = Span::from(format!(
            "{indent}[{}] 📦 ",
            if selected { "✓" } else { " " }
        ))
        .style(style);

        if self.config.show_friendly_names && has_friendly {
            style = style.add_modifier(Modifier::ITALIC);
        }

        let file_name = file.display_stem(self.config.show_friendly_names);
        let content = format!("{}{file_name}", if hovered { "> " } else { "" });

        ListItem::new(Line::from(vec![
            file_prefix_ui,
            Span::from(content).style(style),
        ]))
    }

    fn file_list(&mut self, area: Rect, buf: &mut Buffer) {
        let char_index = self.get_selected_character_index();
        let selected_character = char_index.and_then(|idx| self.characters.get(idx));

        let title = selected_character.map_or_else(
            || Line::styled(" Files ", Style::default().add_modifier(Modifier::BOLD)),
            |character| {
                let style = Style::default().add_modifier(Modifier::BOLD);
                let files_span = Span::from(" Files - ").style(style);
                let char_span = Span::from(format!("{} ", character.name()))
                    .style(style.fg(character.character.class.class_colour()));
                Line::from(vec![files_span, char_span])
            },
        );
        let block = Block::bordered().title(title).border_set(border::THICK);

        let Some(character) = selected_character else {
            Paragraph::new("No character selected")
                .block(block)
                .render(area, buf);
            return;
        };

        let show_highlight = self.input_mode == InputMode::FileSelection;

        let rows = Self::file_rows_for_character(character);

        let items = rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| {
                let hovered = show_highlight
                    && self
                        .file_list_state
                        .selected()
                        .is_some_and(|sel| sel == row_idx);

                match *row {
                    FileRowKind::File(file_idx) => {
                        self.file_row_file_item(character, file_idx, hovered)
                    }
                    FileRowKind::AddonHeader { collapsed, count } => {
                        Self::file_row_addon_header(character, count, collapsed, hovered)
                    }
                    FileRowKind::AddonFile(file_idx) => {
                        self.file_row_addon_item(character, file_idx, hovered)
                    }
                }
            })
            .collect::<Vec<ListItem>>();

        let mut list_view = List::new(items)
            .block(block)
            .style(Style::new().white())
            .repeat_highlight_symbol(true)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
            .direction(ListDirection::TopToBottom);

        if show_highlight {
            list_view =
                list_view.highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG));
        }

        StatefulWidget::render(list_view, area, buf, &mut self.file_list_state);
    }

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
                let prefix = if hovered { "> " } else { "" };
                ListItem::new(format!("{prefix}{text}")).style(style)
            })
            .collect();

        let list = List::new(items).block(block);

        Widget::render(Clear, area, buf);
        Widget::render(list, area, buf);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn bottom_bar(&self, area: Rect, buf: &mut Buffer) {
        let suffix_options = ["q: Quit".to_string()];
        let status_elements = if self.config.show_output {
            vec!["↑/↓: Scroll", "PgUp/PgDn: Fast Scroll", "Home/End: Jump"]
        } else {
            match self.input_mode {
                InputMode::Navigation => vec!["↑/↓: Nav", "↵/→/Space: Select", "b: Backup"],
                InputMode::FileSelection => vec![
                    "↑/↓: Nav",
                    "Space/↵/→: Toggle",
                    "Ctrl+A: Select All",
                    "←: Back",
                    "b: Backup",
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

        let right_line = if let Some((char_idx, _)) = &self.copied_files
            && let Some(copied_char) = self.characters.get(*char_idx)
        {
            let char_text = copied_char.display_name(true);
            Line::from(vec![
                Span::from(" Copying: "),
                Span::from(char_text)
                    .style(Style::default().fg(copied_char.character.class.class_colour())),
            ]).style(Style::default().bg(Color::Black).fg(Color::White))
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

fn handle_character_backup_popup(
    app: &mut ChronoBindApp,
    selected_index: usize,
    option: &str,
    char_idx: usize,
) -> bool {
    let Some(character) = app.characters.get(char_idx) else {
        log::error!("Invalid character index for backup popup: {char_idx}");
        return true;
    };

    let Some(branch_install) = app.find_wow_branch(&character.character.branch) else {
        log::error!(
            "Failed to find WoW installation for branch '{}' during backup",
            character.character.branch
        );
        return true;
    };

    log::info!(
        "Selected backup option: {option} for character {} on branch {}",
        character.name(),
        branch_install.branch_ident
    );

    let backup_result = if selected_index == 0 {
        let selected_files = character.get_all_selected_files();
        backend::backup_character_files(&character.character, &selected_files, branch_install)
    } else if selected_index == 1 {
        backend::backup_character(&character.character, branch_install)
    } else {
        log::error!("Invalid backup option selected: {selected_index}");
        return true; // Close popup on error
    };

    match backup_result {
        Ok(backup_path) => {
            log::info!(
                "Backup completed successfully for character {}. Backup file created at: {}",
                character.name(),
                backup_path.display()
            );
        }
        Err(e) => {
            log::error!("Backup failed for character {}: {}", character.name(), e);
        }
    }

    true
}
