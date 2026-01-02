use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListDirection, ListItem, ListState, Paragraph, Widget};

use crate::Character;
#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::popups::list_with_scrollbar;

/// Represents a row in the file list
#[derive(Debug, Clone, Copy)]
pub enum FileRowKind {
    File(usize),
    AddonHeader { collapsed: bool, count: usize },
    AddonFile(usize),
}

/// Configuration for file list rendering
pub struct FileListConfig {
    pub show_friendly_names: bool,
}

/// The file list widget displays the files for a selected character
#[derive(Debug, Clone)]
pub struct FileListWidget {
    /// The list state for tracking selection
    pub state: ListState,
}

impl Default for FileListWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl FileListWidget {
    /// Icon representing an addon file.
    pub const ADDON_FILE_ICON: &str = "📦";
    /// Icon representing a config file.
    pub const CONFIG_FILE_ICON: &str = "⚙";

    /// Padding value for the file list.
    const PADDING: usize = 1;

    /// Create a new file list widget
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
        }
    }

    /// Generate the list of file rows for a character
    #[must_use]
    pub fn file_rows_for_character(character: &Character) -> Vec<FileRowKind> {
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

    /// Handle input for the file list in file selection mode
    /// Returns the action to be taken
    pub fn handle_file_selection_input(
        &mut self,
        key: &KeyEvent,
        character: &mut Character,
    ) -> FileSelectionAction {
        let rows = Self::file_rows_for_character(character);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Char('a' | 'A') if !ctrl => FileSelectionAction::ExitFileSelection,
            KeyCode::Esc | KeyCode::Left => FileSelectionAction::ExitFileSelection,
            KeyCode::Up | KeyCode::Char('w' | 'W') => {
                if let Some(sel_index) = self.state.selected() {
                    self.state.select(Some(sel_index.saturating_sub(1)));
                }
                FileSelectionAction::None
            }
            KeyCode::Down | KeyCode::Char('s' | 'S') => {
                if let Some(sel_index) = self.state.selected() {
                    self.state.select(Some(sel_index + 1));
                }
                FileSelectionAction::None
            }
            KeyCode::Char(' ' | 'd' | 'D') | KeyCode::Enter | KeyCode::Right => {
                let Some(selected_index) = self.state.selected() else {
                    return FileSelectionAction::None;
                };
                if selected_index < rows.len() {
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
                                log::debug!(
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
                            log::debug!("Addon file '{file_name}' toggled: {selected}");
                        }
                    }
                }
                FileSelectionAction::None
            }
            KeyCode::Char('a' | 'A') if ctrl => {
                let all_selected =
                    character.all_config_files_selected() && character.all_addon_files_selected();
                character.set_all_selected(!all_selected);
                log::debug!(
                    "All files {}",
                    if all_selected {
                        "deselected"
                    } else {
                        "selected"
                    }
                );
                FileSelectionAction::None
            }
            KeyCode::Char('b' | 'B') => FileSelectionAction::ShowBackup,
            KeyCode::Char('c' | 'C') => FileSelectionAction::Copy,
            _ => FileSelectionAction::None,
        }
    }

    /// Render a file row item
    fn file_row_file_item<'a>(
        character: &Character,
        file_idx: usize,
        hovered: bool,
        config: &FileListConfig,
    ) -> ListItem<'a> {
        let file = &character.config_files()[file_idx];
        let selected = character.selected_config_files[file_idx];
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            SELECTED_FG
        } else if has_friendly && config.show_friendly_names {
            SPECIAL_FG
        } else {
            STD_FG
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui = Span::from(format!(
            "{pad}{} {}  ",
            checkbox(selected),
            Self::CONFIG_FILE_ICON,
            pad = indentation(Self::PADDING)
        ))
        .style(style);

        if config.show_friendly_names && has_friendly {
            style = style.add_modifier(Modifier::ITALIC);
        }

        let file_name = file.display_name(config.show_friendly_names);
        let content = format!("{}{file_name}", highlight_symbol(hovered));

        ListItem::new(Line::from(vec![
            file_prefix_ui,
            Span::from(content).style(style),
        ]))
    }

    /// Render an addon header row
    fn file_row_addon_header(
        character: &Character,
        count: usize,
        collapsed: bool,
        hovered: bool,
    ) -> ListItem<'_> {
        let no_addon_files = character.addon_files().is_empty();
        let any_addon_file_selected = character.any_addon_file_selected();
        let all_addon_file_selected = character.all_addon_files_selected();

        let colour = if all_addon_file_selected && !no_addon_files {
            SELECTED_FG
        } else if any_addon_file_selected && !no_addon_files {
            Color::Yellow
        } else {
            STD_FG
        };

        let label = format!("Addon Options ({count})");
        let content = format!(
            "{pad}{} {}{label}",
            expandable_icon(collapsed),
            highlight_symbol(hovered),
            pad = indentation(Self::PADDING)
        );

        let dropdown_style = Style::default()
            .fg(colour)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::ITALIC);

        ListItem::new(Line::from(content).style(dropdown_style))
    }

    /// Render an addon file row item
    fn file_row_addon_item<'a>(
        character: &Character,
        file_idx: usize,
        hovered: bool,
        config: &FileListConfig,
    ) -> ListItem<'a> {
        const ADDON_IDENT: usize = 3;

        let selected = character.selected_addon_files[file_idx];
        let file = &character.addon_files()[file_idx];
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            SELECTED_FG
        } else if has_friendly && config.show_friendly_names {
            SPECIAL_FG
        } else {
            STD_FG
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui = Span::from(format!(
            "{pad}{} {} ",
            checkbox(selected),
            Self::ADDON_FILE_ICON,
            pad = indentation(Self::PADDING + ADDON_IDENT)
        ))
        .style(style);

        if config.show_friendly_names && has_friendly {
            style = style.add_modifier(Modifier::ITALIC);
        }

        let file_name = file.display_stem(config.show_friendly_names);
        let content = format!("{}{file_name}", highlight_symbol(hovered));

        ListItem::new(Line::from(vec![
            file_prefix_ui,
            Span::from(content).style(style),
        ]))
    }

    /// Render the file list widget
    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        character: Option<&Character>,
        show_highlight: bool,
        config: &FileListConfig,
    ) {
        let title = character.map_or_else(
            || Line::styled(" Files ", Style::default().add_modifier(Modifier::BOLD)),
            |character| {
                let style = Style::default().add_modifier(Modifier::BOLD);
                let files_span = Span::from(" Files - ").style(style);
                let char_span = character.display_span(true);
                Line::from(vec![files_span, char_span, Span::from(" ")])
            },
        );
        let block = Block::bordered().title(title).border_set(border::THICK);

        let Some(character) = character else {
            Paragraph::new(format!(
                "{pad}No character selected",
                pad = indentation(Self::PADDING)
            ))
            .block(block)
            .render(area, buf);
            return;
        };

        let rows = Self::file_rows_for_character(character);

        let items = rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| {
                let hovered =
                    show_highlight && self.state.selected().is_some_and(|sel| sel == row_idx);

                match *row {
                    FileRowKind::File(file_idx) => {
                        Self::file_row_file_item(character, file_idx, hovered, config)
                    }
                    FileRowKind::AddonHeader { collapsed, count } => {
                        Self::file_row_addon_header(character, count, collapsed, hovered)
                    }
                    FileRowKind::AddonFile(file_idx) => {
                        Self::file_row_addon_item(character, file_idx, hovered, config)
                    }
                }
            })
            .collect::<Vec<ListItem>>();

        let mut list_view = List::new(items)
            .block(block)
            .style(Style::new().white())
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
            .direction(ListDirection::TopToBottom);

        if show_highlight {
            list_view =
                list_view.highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG));
        }

        list_with_scrollbar(list_view, area, buf, &mut self.state);
    }
}

/// Action to be taken after handling file selection input
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileSelectionAction {
    /// No action needed
    None,
    /// Exit file selection mode
    ExitFileSelection,
    /// Show backup popup
    ShowBackup,
    /// Copy selected files
    Copy,
}
