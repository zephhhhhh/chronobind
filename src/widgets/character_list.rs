use std::collections::{BTreeMap, BTreeSet};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListDirection, ListItem, ListState};

use crate::config::ChronoBindAppConfig;
use crate::ui::{Character, KeyCodeExt};

#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::popups::list_with_scrollbar;

/// Represents a row in the file list
#[derive(Debug, Clone)]
pub enum CharacterListItemKind {
    Character(usize),
    RealmHeader {
        realm_ident: String,
        collapsed: bool,
        count: usize,
    },
}

/// The character list widget displays the characters grouped by realm with collapsible headers.
#[derive(Debug, Clone)]
pub struct CharacterListWidget {
    /// The branch display string (e.g., "Retail", "Classic", etc.)
    pub branch_display: Option<String>,
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
            branch_display: None,
            state: ListState::default(),
            collapsed_realms: BTreeSet::new(),
        }
    }

    /// Get the currently selected index in the list
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    /// Generate the list of character list items with realm grouping
    #[inline]
    #[must_use]
    pub fn get_character_list_items(&self, characters: &[Character]) -> Vec<CharacterListItemKind> {
        let mut realms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, character) in characters.iter().enumerate() {
            realms
                .entry(character.realm().to_string())
                .or_default()
                .push(i);
        }

        let mut items = Vec::with_capacity(characters.len() + realms.len());
        for (realm, char_indices) in &realms {
            let collapsed = self.collapsed_realms.contains(realm);
            let count = char_indices.len();

            // Add realm header
            items.push(CharacterListItemKind::RealmHeader {
                realm_ident: realm.clone(),
                collapsed,
                count,
            });

            if !collapsed {
                items.extend_from_slice(
                    &char_indices
                        .iter()
                        .map(|c| CharacterListItemKind::Character(*c))
                        .collect::<Vec<_>>(),
                );
            }
        }

        items
    }

    /// Get the actual character index from the selected position, accounting for grouped display
    #[must_use]
    pub fn get_selected_character_index_from_chars(&self, chars: &[Character]) -> Option<usize> {
        let item_list = self.get_character_list_items(chars);
        self.get_selected_character_index(&item_list)
    }

    /// Get the actual character index from the selected position, accounting for grouped display
    #[must_use]
    pub fn get_selected_character_index(
        &self,
        item_list: &[CharacterListItemKind],
    ) -> Option<usize> {
        let selected_index = self.selected_index();
        match item_list.get(selected_index) {
            Some(CharacterListItemKind::Character(char_idx)) => Some(*char_idx),
            _ => None,
        }
    }

    /// Handle input for the character list in navigation mode
    /// Returns true if the input mode should change to file selection
    pub fn handle_navigation_input(
        &mut self,
        key: &KeyEvent,
        characters: &[Character],
    ) -> NavigationAction {
        let item_list = self.get_character_list_items(characters);

        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
                NavigationAction::None
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
                NavigationAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ' | 'd') | KeyCode::Right => {
                match item_list.get(self.selected_index()) {
                    Some(CharacterListItemKind::RealmHeader { realm_ident, .. }) => {
                        // Toggle realm collapse
                        if self.collapsed_realms.contains(realm_ident) {
                            self.collapsed_realms.remove(realm_ident);
                        } else {
                            self.collapsed_realms.insert(realm_ident.clone());
                        }
                        NavigationAction::None
                    }
                    Some(CharacterListItemKind::Character(_)) => {
                        // Character selected, enter file selection
                        log::debug!("Entered file selection mode");
                        NavigationAction::EnterFileSelection
                    }
                    None => NavigationAction::None,
                }
            }
            KeyCode::Char('b') => {
                if let Some(CharacterListItemKind::Character(char_idx)) =
                    item_list.get(self.selected_index())
                {
                    NavigationAction::ShowBackup(*char_idx)
                } else {
                    NavigationAction::None
                }
            }
            KeyCode::Char('c') => self
                .get_selected_character_index(&item_list)
                .map_or(NavigationAction::None, |char_idx| {
                    NavigationAction::Copy(char_idx)
                }),
            KeyCode::Char('v') => self
                .get_selected_character_index(&item_list)
                .map_or(NavigationAction::None, |target_char_idx| {
                    NavigationAction::Paste(target_char_idx)
                }),
            _ => NavigationAction::None,
        }
    }

    /// Render the character list widget
    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        characters: &[Character],
        config: &ChronoBindAppConfig,
    ) {
        const PADDING: usize = 1;
        const INDENT: usize = 3;

        let title_content = self.branch_display.as_ref().map_or_else(
            || " Characters ".to_string(),
            |branch| format!(" Characters - {branch} "),
        );
        let block = Block::bordered()
            .title(Line::from(title_content).bold())
            .border_set(border::THICK);

        let char_list_items = self.get_character_list_items(characters);

        let selected_index = self.selected_index();
        let items = char_list_items
            .iter()
            .enumerate()
            .map(|(i, li)| {
                let hovered = i == selected_index;
                match li {
                    CharacterListItemKind::RealmHeader {
                        realm_ident,
                        collapsed,
                        ..
                    } => {
                        let content = format!(
                            "{pad}{} {}[{realm_ident}]",
                            expandable_icon(*collapsed),
                            highlight_symbol(hovered),
                            pad = indentation(PADDING)
                        );
                        ListItem::new(content).bold().fg(PALETTE.std_fg).dim()
                    }
                    CharacterListItemKind::Character(char_idx) => {
                        let character = &characters[*char_idx];
                        let files_selected = character.any_file_selected();

                        let ui_span_text = format!(
                            "{pad}{}",
                            highlight_symbol(hovered),
                            pad = indentation(PADDING + INDENT)
                        );
                        let ui_span_source = if files_selected {
                            Span::from(format!("{ui_span_text}• ")).fg(PALETTE.selected_fg)
                        } else {
                            Span::from(ui_span_text)
                        };

                        let main_span = if config.display_character_levels {
                            character.display_span_with_meta(false)
                        } else {
                            character.display_span(false)
                        };

                        ListItem::new(Line::from(vec![ui_span_source, main_span]))
                    }
                }
            })
            .collect::<Vec<_>>();

        let list_view = List::new(items)
            .block(block)
            .fg(PALETTE.std_fg)
            .highlight_style(Style::new().bold().bg(PALETTE.hover_bg))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
            .direction(ListDirection::TopToBottom);

        list_with_scrollbar(list_view, area, buf, &mut self.state);
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
