use std::collections::{BTreeMap, BTreeSet};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListDirection, ListItem, ListState, StatefulWidget};

use crate::Character;
#[allow(clippy::wildcard_imports)]
use crate::palette::*;

/// The character list widget displays the characters grouped by realm with collapsible headers.
#[derive(Debug, Clone)]
pub struct CharacterListWidget {
    /// The list state for tracking selection
    pub state: ListState,
    /// Set of collapsed realm names
    pub collapsed_realms: BTreeSet<String>,
}

impl Default for CharacterListWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterListWidget {
    /// Create a new character list widget
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            collapsed_realms: BTreeSet::new(),
        }
    }

    /// Get the currently selected index in the list
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    /// Get the actual character index from the selected position, accounting for grouped display
    #[must_use]
    pub fn get_selected_character_index(&self, characters: &[Character]) -> Option<usize> {
        // Build the grouped structure
        let mut realms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, character) in characters.iter().enumerate() {
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

    /// Handle input for the character list in navigation mode
    /// Returns true if the input mode should change to file selection
    pub fn handle_navigation_input(
        &mut self,
        key: &KeyEvent,
        characters: &[Character],
    ) -> NavigationAction {
        // Build the grouped structure to determine navigation
        let mut realms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, character) in characters.iter().enumerate() {
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
            KeyCode::Up | KeyCode::Char('w' | 'W') => {
                if let Some(selected) = self.state.selected() {
                    self.state.select(Some(selected.saturating_sub(1)));
                }
                NavigationAction::None
            }
            KeyCode::Down | KeyCode::Char('s' | 'S') => {
                if let Some(selected) = self.state.selected() {
                    self.state.select(Some(selected.saturating_add(1)));
                }
                NavigationAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some((_, is_header, realm_or_idx)) = abs_positions.get(self.selected_index())
                {
                    if *is_header {
                        // Toggle realm collapse
                        if self.collapsed_realms.contains(realm_or_idx) {
                            self.collapsed_realms.remove(realm_or_idx);
                        } else {
                            self.collapsed_realms.insert(realm_or_idx.clone());
                        }
                        NavigationAction::None
                    } else {
                        // Character selected, enter file selection
                        log::debug!("Entered file selection mode");
                        NavigationAction::EnterFileSelection
                    }
                } else {
                    NavigationAction::None
                }
            }
            KeyCode::Char('d' | 'D') | KeyCode::Right => {
                if let Some((_, is_header, _)) = abs_positions.get(self.selected_index())
                    && !*is_header
                {
                    log::debug!("Entered file selection mode");
                    NavigationAction::EnterFileSelection
                } else {
                    NavigationAction::None
                }
            }
            KeyCode::Char('b' | 'B') => {
                if let Some((_, is_header, _)) = abs_positions.get(self.selected_index())
                    && !*is_header
                    && let Some(char_idx) = self.get_selected_character_index(characters)
                {
                    NavigationAction::ShowBackup(char_idx)
                } else {
                    NavigationAction::None
                }
            }
            KeyCode::Char('c' | 'C') => self
                .get_selected_character_index(characters)
                .map_or(NavigationAction::None, |char_idx| {
                    NavigationAction::Copy(char_idx)
                }),
            KeyCode::Char('v' | 'V') => self
                .get_selected_character_index(characters)
                .map_or(NavigationAction::None, |target_char_idx| {
                    NavigationAction::Paste(target_char_idx)
                }),
            _ => NavigationAction::None,
        }
    }

    /// Render the character list widget
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, characters: &[Character]) {
        const PADDING_VALUE: usize = 1;
        const INDENT_DEPTH: usize = 3;
        let indent = indentation(INDENT_DEPTH);
        let padding = indentation(PADDING_VALUE);

        let title = Line::styled(
            " Characters ",
            Style::default().add_modifier(Modifier::BOLD),
        );
        let block = Block::bordered().title(title).border_set(border::THICK);

        let mut realms: BTreeMap<String, Vec<(usize, &Character)>> = BTreeMap::new();
        for (i, character) in characters.iter().enumerate() {
            realms
                .entry(character.realm().to_string())
                .or_default()
                .push((i, character));
        }

        let mut items = Vec::new();

        for (realm, chars) in &realms {
            // Add realm header
            let is_collapsed = self.collapsed_realms.contains(realm);
            let hovered = self.state.selected().is_some_and(|sel| sel == items.len());
            let header_style = Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(STD_FG)
                .add_modifier(Modifier::DIM);
            let content = format!(
                "{}{} {}[{realm}]",
                padding,
                expandable_icon(is_collapsed),
                highlight_symbol(hovered)
            );
            items.push(ListItem::new(content).style(header_style));

            // Add characters in this realm (only if not collapsed)
            if !is_collapsed {
                for (_, character) in chars {
                    let hovered = self.state.selected().is_some_and(|sel| sel == items.len());
                    let style = Style::default();

                    let files_selected = character.any_file_selected();
                    let colour = character.character.class.class_colour();

                    let ui_span_text = format!("{}{indent}{}", padding, highlight_symbol(hovered));
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

        StatefulWidget::render(list_view, area, buf, &mut self.state);
    }
}

/// Action to be taken after handling navigation input
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NavigationAction {
    /// No action needed
    None,
    /// Enter file selection mode
    EnterFileSelection,
    /// Show backup popup for the given character index
    ShowBackup(usize),
    /// Copy files from the given character index
    Copy(usize),
    /// Paste files to the given character index
    Paste(usize),
}
