#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

pub mod tui_log;
pub mod wow;

use std::collections::BTreeMap;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::Context;
use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};

use crate::wow::WowCharacter;

// Colours..
const DARK_SLATE: Color = Color::Rgb(22, 31, 31);
const SELECTED_GREEN: Color = Color::Rgb(30, 143, 32);

const SPECIAL_WHITE: Color = Color::Rgb(205, 232, 250);

/// Convert an (r, g, b) tuple into a `Color::Rgb`
#[inline]
#[must_use]
const fn into_colour((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

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
struct ChronoBindAppConfig {
    pub show_realm: bool,
    pub show_output: bool,
    pub group_by_realm: bool,
    pub show_friendly_names: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum InputMode {
    #[default]
    Navigation,
    FileSelection,
}

#[derive(Debug, Default)]
struct ChronoBindApp {
    should_exit: bool,
    #[allow(dead_code)]
    wow_installations: Vec<wow::WowInstall>,
    characters: Vec<Character>,
    selected_index: usize,
    selected_file_index: usize,
    input_mode: InputMode,
    config: ChronoBindAppConfig,
    debug_scroll_offset: usize,
    collapsed_realms: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct Character {
    pub character: WowCharacter,
    selected_files: Vec<bool>,
}

impl Character {
    pub fn new(character: &WowCharacter) -> Self {
        let file_count = character.files.len();
        Self {
            character: character.clone(),
            selected_files: vec![false; file_count],
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
    pub fn files(&self) -> &[wow::WowCharacterFile] {
        &self.character.files
    }
}

impl ChronoBindApp {
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
            selected_index: 0,
            selected_file_index: 0,
            debug_scroll_offset: 0,
            input_mode: InputMode::Navigation,
            collapsed_realms: std::collections::BTreeSet::new(),
            config: ChronoBindAppConfig {
                show_realm: false,
                show_output: false,
                group_by_realm: true,
                show_friendly_names: true,
            },
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
        self.selected_index = 0;
        self.selected_file_index = 0;
    }

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
                self.config.group_by_realm = !self.config.group_by_realm;
                self.selected_index = 0;
                self.selected_file_index = 0;
            }
            KeyCode::F(3) => {
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
        if self.config.group_by_realm {
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
                    if self.selected_index > 0 {
                        self.selected_index = self.selected_index.saturating_sub(1);
                        self.selected_file_index = 0;
                    }
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    if self.selected_index < abs_positions.len() - 1 {
                        self.selected_index += 1;
                        self.selected_file_index = 0;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some((_, is_header, realm_or_idx)) =
                        abs_positions.get(self.selected_index)
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
                            self.selected_file_index = 0;
                            log::debug!("Entered file selection mode");
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Right => {
                    if let Some((_, is_header, _)) = abs_positions.get(self.selected_index)
                        && !*is_header
                    {
                        self.input_mode = InputMode::FileSelection;
                        self.selected_file_index = 0;
                        log::debug!("Entered file selection mode");
                    }
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Char('w') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        self.selected_file_index = 0;
                    }
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    if self.selected_index < self.characters.len().saturating_sub(1) {
                        self.selected_index += 1;
                        self.selected_file_index = 0;
                    }
                }
                KeyCode::Enter | KeyCode::Char('d' | ' ') | KeyCode::Right => {
                    self.input_mode = InputMode::FileSelection;
                    self.selected_file_index = 0;
                    log::debug!("Entered file selection mode");
                }
                _ => {}
            }
        }
    }

    fn handle_file_selection_keys(&mut self, key: &KeyEvent) {
        let char_index = self.get_selected_character_index();
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
                if self.selected_file_index > 0 {
                    self.selected_file_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(character) = character
                    && self.selected_file_index < character.files().len().saturating_sub(1)
                {
                    self.selected_file_index += 1;
                }
            }
            KeyCode::Char(' ' | 'd') | KeyCode::Enter | KeyCode::Right => {
                if let Some(character) = character
                    && self.selected_file_index < character.selected_files.len()
                {
                    character.selected_files[self.selected_file_index] =
                        !character.selected_files[self.selected_file_index];
                    let file_name = character.files()[self.selected_file_index].get_full_filename();
                    let selected = character.selected_files[self.selected_file_index];
                    log::info!("File '{file_name}' toggled: {selected}");
                }
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(character) = character {
                    let all_selected = character.selected_files.iter().all(|&s| s);
                    if all_selected {
                        character.selected_files.fill(false);
                        log::debug!("All files deselected");
                    } else {
                        character.selected_files.fill(true);
                        log::debug!("All files selected");
                    }
                }
            }
            _ => {}
        }
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
        if self.config.group_by_realm {
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
                        if current_pos == self.selected_index {
                            return Some(char_idx);
                        }
                        current_pos += 1;
                    }
                }
            }
            None
        } else {
            Some(self.selected_index)
        }
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

    fn flat_character_items(&self) -> Vec<ListItem<'_>> {
        self.characters
            .iter()
            .enumerate()
            .map(|(i, character)| {
                let hovered = self.selected_index == i;
                let mut style = Style::default();
                if hovered {
                    style = style.add_modifier(Modifier::BOLD);
                }

                let files_selected = character.selected_files.iter().any(|s| *s);
                let colour = into_colour(character.character.class.class_colour());

                let ui_span_text = if hovered { "> " } else { "" };
                let ui_span_source = if files_selected {
                    Span::from(format!("{ui_span_text}• ")).style(style.fg(SELECTED_GREEN))
                } else {
                    Span::from(ui_span_text).style(style)
                };

                let main_span = Span::from(character.display_name(self.config.show_realm))
                    .style(style.fg(colour));

                let all_style = style.bg(if hovered { DARK_SLATE } else { Color::Reset });

                ListItem::new(Line::from(vec![ui_span_source, main_span])).style(all_style)
            })
            .collect()
    }

    fn realm_grouped_character_items(&self) -> Vec<ListItem<'_>> {
        const INDENT_DEPTH: usize = 3;
        let indentation = " ".repeat(INDENT_DEPTH);

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
            let hovered = self.selected_index == items.len();
            let collapse_icon = if is_collapsed { "▶" } else { "▼" };
            let mut header_style = Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Gray);
            if hovered {
                header_style = header_style.bg(DARK_SLATE);
            }
            let content = format!(
                "{collapse_icon} {}[{realm}]",
                if hovered { "> " } else { "" }
            );
            items.push(ListItem::new(content).style(header_style));

            // Add characters in this realm (only if not collapsed)
            if !is_collapsed {
                for (_, character) in chars {
                    let hovered = self.selected_index == items.len();
                    let style = Style::default();

                    let files_selected = character.selected_files.iter().any(|s| *s);
                    let colour = into_colour(character.character.class.class_colour());

                    let ui_span_text = format!("{indentation}{}", if hovered { "> " } else { "" });
                    let ui_span_source = if files_selected {
                        Span::from(format!("{ui_span_text}• ")).style(style.fg(SELECTED_GREEN))
                    } else {
                        Span::from(ui_span_text).style(style)
                    };
                    let main_span = Span::from(character.name()).style(style.fg(colour));

                    let all_style = style.bg(if hovered { DARK_SLATE } else { Color::Reset });

                    items.push(
                        ListItem::new(Line::from(vec![ui_span_source, main_span])).style(all_style),
                    );
                }
            }
        }

        items
    }

    fn character_list(&self, area: Rect, buf: &mut Buffer) {
        let title = Line::styled(
            " Characters ",
            Style::default().add_modifier(Modifier::BOLD),
        );

        let block = Block::bordered().title(title).border_set(border::THICK);

        let items = if self.config.group_by_realm {
            self.realm_grouped_character_items()
        } else {
            self.flat_character_items()
        };

        let list = List::new(items).block(block);
        Widget::render(list, area, buf);
    }

    fn file_list(&self, area: Rect, buf: &mut Buffer) {
        let char_index = self.get_selected_character_index();
        let selected_character = char_index.and_then(|idx| self.characters.get(idx));

        let title = selected_character.map_or_else(
            || Line::styled(" Files ", Style::default().add_modifier(Modifier::BOLD)),
            |character| {
                let style = Style::default().add_modifier(Modifier::BOLD);
                let files_span = Span::from(" Files - ").style(style);
                let char_span = Span::from(format!("{} ", character.name()))
                    .style(style.fg(into_colour(character.character.class.class_colour())));
                Line::from(vec![files_span, char_span])
            },
        );

        let block = Block::bordered().title(title).border_set(border::THICK);

        if let Some(character) = selected_character {
            // Create list items from files with selection indicators
            let items: Vec<ListItem> = character
                .files()
                .iter()
                .enumerate()
                .map(|(i, file)| {
                    let hovered = self.input_mode == InputMode::FileSelection
                        && self.selected_file_index == i;
                    let selected = character.selected_files[i];
                    let has_friendly = file.has_friendly_name();

                    let fg_colour = if selected {
                        SELECTED_GREEN
                    } else if has_friendly && self.config.show_friendly_names {
                        SPECIAL_WHITE
                    } else {
                        Color::White
                    };

                    let mut style = Style::default().fg(fg_colour);

                    let file_prefix_ui =
                        Span::from(format!("[{}] 📄 ", if selected { "✓" } else { " " }))
                            .style(style);

                    if self.config.show_friendly_names && has_friendly {
                        style = style.add_modifier(Modifier::ITALIC);
                    }

                    let file_name = file.display_name(self.config.show_friendly_names);
                    let content = format!(
                        "{}{file_name}",
                        if self.input_mode == InputMode::FileSelection && hovered {
                            "> "
                        } else {
                            ""
                        }
                    );

                    let mut all_style = Style::default();
                    if hovered {
                        all_style = all_style.add_modifier(Modifier::BOLD);
                        all_style = all_style.bg(DARK_SLATE);
                    }

                    ListItem::new(Line::from(vec![
                        file_prefix_ui,
                        Span::from(content).style(style),
                    ]))
                    .style(all_style)
                })
                .collect();

            let list = List::new(items).block(block);

            Widget::render(list, area, buf);
        } else {
            Paragraph::new("No character selected")
                .block(block)
                .render(area, buf);
        }
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
                    let color = match log.level() {
                        log::Level::Error => Color::Red,
                        log::Level::Warn => Color::Yellow,
                        log::Level::Info => Color::Blue,
                        log::Level::Debug => Color::Cyan,
                        log::Level::Trace => Color::Gray,
                    };
                    Line::from(log.content().to_string()).style(Style::default().fg(color))
                })
                .collect()
        });

        let log_text = log_lines.unwrap_or_else(|| {
            vec![Line::from("Failed to retrieve logs").style(Style::default().fg(Color::Red))]
        });

        Paragraph::new(log_text).block(block).render(area, buf);
    }

    fn bottom_bar(&self, area: Rect, buf: &mut Buffer) {
        let suffix_options = ["q: Quit".to_string()];
        let status_elements = if self.config.show_output {
            vec!["↑/↓: Scroll", "PgUp/PgDn: Fast Scroll", "Home/End: Jump"]
        } else {
            match self.input_mode {
                InputMode::Navigation => vec!["↑/↓: Navigate", "↵/→/Space: Select"],
                InputMode::FileSelection => vec![
                    "↑/↓: Navigate",
                    "Space/↵/→: Toggle",
                    "Ctrl+A: Select All",
                    "←: Characters",
                ],
            }
        };

        let final_text = status_elements
            .iter()
            .map(std::string::ToString::to_string)
            .chain(suffix_options)
            .join(" | ");

        let status_line = Line::from(Span::styled(final_text, Style::default()))
            .style((Color::Black, Color::White));

        status_line.render(area, buf);
    }
}
