#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    CharacterWithIndex,
    popups::list_with_scrollbar,
    widgets::popup::{Popup, PopupCommand},
};

use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, List, ListDirection, ListItem, ListState, Padding},
};

/// Different commands that can be issued from a restore popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RestorePopupCommand {
    /// Command to restore a backup at a specified index.
    RestoreBackup(usize),
}

/// Popup for restoring a backup for a character.
#[derive(Debug, Clone)]
pub struct RestorePopup {
    /// The character associated with the restore popup.
    pub dest_char: CharacterWithIndex,
    /// The copied character if applicable, for restoring from their backups.
    pub source_char: Option<CharacterWithIndex>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl RestorePopup {
    #[must_use]
    pub fn new(character: CharacterWithIndex, copied_char: Option<CharacterWithIndex>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            dest_char: character,
            source_char: copied_char,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Create a popup command for the given restore command.
    #[inline]
    #[must_use]
    pub const fn get_command(&self, command: RestorePopupCommand) -> PopupCommand {
        PopupCommand::Restore(self.dest_char.1, command)
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: PopupCommand) {
        self.commands.push(command);
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: PopupCommand) {
        self.push_command(command);
        self.close = true;
    }

    /// If the source character is set, return `true`.
    #[inline]
    #[must_use]
    pub const fn has_source_char(&self) -> bool {
        self.source_char.is_some()
    }

    /// Get the source character, or the destination character if no source is set.
    #[inline]
    #[must_use]
    pub fn source_char(&self) -> &CharacterWithIndex {
        self.source_char.as_ref().unwrap_or(&self.dest_char)
    }

    /// Get the backup at a specified index from the source character.
    #[inline]
    #[must_use]
    pub fn get_backup(&self, index: usize) -> Option<&crate::wow::WowBackup> {
        self.source_char().0.backups().get(index)
    }
}

impl Popup for RestorePopup {
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('w' | 'W') => {
                self.state
                    .select(self.state.selected().map(|i| i.saturating_sub(1)));
            }
            KeyCode::Down | KeyCode::Char('s' | 'S') => {
                self.state
                    .select(self.state.selected().map(|i| i.saturating_add(1)));
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selected) = self.state.selected()
                    && let Some(backup) = self.get_backup(selected).cloned()
                {
                    let command = self.get_command(RestorePopupCommand::RestoreBackup(selected));
                    let start_span =
                        Span::from(format!("Restore backup `{}` to ", backup.formatted_name()));
                    let dest_char_span = self.dest_char.0.display_span(true).bold();
                    self.push_command_close(
                        command.with_confirm_and_line(Line::from(vec![start_span, dest_char_span])),
                    );
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let title_spans = vec![
            Span::from(" Restore "),
            self.dest_char.0.display_span(true),
            Span::from(" "),
        ];
        let title_style = Style::default().add_modifier(Modifier::BOLD);

        let block = Block::bordered()
            .title(Line::from(title_spans).style(title_style))
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black))
            .padding(Padding::symmetric(1, 0));

        let items = self
            .source_char()
            .0
            .backups()
            .iter()
            .enumerate()
            .map(|(i, backup)| {
                let hovered = i == self.state.selected().unwrap_or(0);
                let content = format!(
                    "{}{} {}{}",
                    pinned_string(backup.is_pinned),
                    backup.char_name,
                    display_backup_time(&backup.timestamp),
                    if backup.is_paste { " (Auto)" } else { "" },
                );
                let line = Line::from(dual_highlight_str(content, hovered)).centered();
                ListItem::new(line)
            })
            .collect_vec();

        let list_view = List::new(items)
            .block(block)
            .style(Style::new().white())
            .highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG))
            .direction(ListDirection::TopToBottom);

        list_with_scrollbar(list_view, area, buf, &mut self.state);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "restore_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        Some(vec!["↑/↓", "↵/Space: Select", "Esc: Close"])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        Some(&mut self.commands)
    }
}
