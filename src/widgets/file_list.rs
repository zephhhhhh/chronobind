use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListDirection, ListItem, ListState, Paragraph, Widget};

use crate::ui::{Character, KeyCodeExt};

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

        match key.keycode_lower() {
            KeyCode::Char('a') if !ctrl => FileSelectionAction::ExitFileSelection,
            KeyCode::Esc | KeyCode::Left => FileSelectionAction::ExitFileSelection,
            KeyCode::Up | KeyCode::Char('w') => {
                if let Some(sel_index) = self.state.selected() {
                    self.state.select(Some(sel_index.saturating_sub(1)));
                }
                FileSelectionAction::None
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if let Some(sel_index) = self.state.selected() {
                    self.state.select(Some(sel_index + 1));
                }
                FileSelectionAction::None
            }
            KeyCode::Char(' ' | 'd') | KeyCode::Enter | KeyCode::Right => {
                let Some(selected_index) = self.state.selected() else {
                    return FileSelectionAction::None;
                };
                if selected_index < rows.len() {
                    match rows[selected_index] {
                        FileRowKind::File(idx) => {
                            if ctrl {
                                let selected = character.all_config_files_selected();
                                character.set_all_config_selected(!selected);
                                log::info!(
                                    "{} all config files",
                                    if selected { "Deselected" } else { "Selected" }
                                );
                            } else {
                                let selected = character.toggle_config_file_selected(idx);
                                let file_name = character.config_files()[idx].get_full_filename();
                                log::info!("File '{file_name}' toggled: {selected}");
                            }
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
                            if ctrl {
                                let selected = character.all_addon_files_selected();
                                character.set_all_addon_selected(!selected);
                                log::debug!(
                                    "{} all addon files",
                                    if selected { "Deselected" } else { "Selected" }
                                );
                            } else {
                                let selected = character.toggle_addon_file_selected(idx);
                                let file_name = character.addon_files()[idx].get_full_filename();
                                log::debug!("Addon file '{file_name}' toggled: {selected}");
                            }
                        }
                    }
                }
                FileSelectionAction::None
            }
            KeyCode::Char('a') if ctrl => {
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
            KeyCode::Char('b') => FileSelectionAction::ShowBackup,
            KeyCode::Char('c') => FileSelectionAction::Copy,
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
        let selected = character.is_config_file_selected(file_idx);
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            PALETTE.selected_fg
        } else if has_friendly && config.show_friendly_names {
            PALETTE.special_fg
        } else {
            PALETTE.std_fg
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui = Span::from(format!(
            "{pad}{} {} ",
            checkbox(selected),
            *CONFIG_FILE_ICON,
            pad = indentation(Self::PADDING)
        ))
        .style(style);

        if config.show_friendly_names && has_friendly {
            style = style.italic();
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
            PALETTE.selected_fg
        } else if any_addon_file_selected && !no_addon_files {
            PALETTE.log_warn_fg
        } else {
            PALETTE.std_fg
        };

        let label = format!("Addon Options ({count})");
        let content = format!(
            "{pad}{} {}{label}",
            expandable_icon(collapsed),
            highlight_symbol(hovered),
            pad = indentation(Self::PADDING)
        );

        ListItem::new(Line::from(content).fg(colour).bold().italic())
    }

    /// Render an addon file row item
    fn file_row_addon_item<'a>(
        character: &Character,
        file_idx: usize,
        hovered: bool,
        config: &FileListConfig,
    ) -> ListItem<'a> {
        const ADDON_IDENT: usize = 3;

        let selected = character.is_addon_file_selected(file_idx);
        let file = &character.addon_files()[file_idx];
        let has_friendly = file.has_friendly_name();

        let fg_colour = if selected {
            PALETTE.selected_fg
        } else if has_friendly && config.show_friendly_names {
            PALETTE.special_fg
        } else {
            PALETTE.std_fg
        };
        let mut style = Style::default().fg(fg_colour);

        let file_prefix_ui = Span::from(format!(
            "{pad}{} {} ",
            checkbox(selected),
            *ADDON_FILE_ICON,
            pad = indentation(Self::PADDING + ADDON_IDENT)
        ))
        .style(style);

        if config.show_friendly_names && has_friendly {
            style = style.italic();
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
            || Line::from(" Files ").bold(),
            |character| {
                let files_span = Span::from(" Files - ").bold();
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
            .fg(PALETTE.std_fg)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
            .direction(ListDirection::TopToBottom);

        if show_highlight {
            list_view = list_view.highlight_style(Style::new().bold().bg(PALETTE.hover_bg));
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
