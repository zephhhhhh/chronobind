#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    CharacterWithIndex,
    popups::list_with_scrollbar,
    widgets::popup::{Popup, PopupCommand, PopupMessage},
};

use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, List, ListDirection, ListItem, ListState, Padding},
};

/// Different commands that can be issued from a backup manager popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BackupManagerPopupCommand {
    ToggleBackupPin(usize),
    DeleteBackup(usize),
}

/// Popup for managing backups for a character.
#[derive(Debug, Clone)]
pub struct BackupManagerPopup {
    /// The character associated with the backup manager popup.
    pub character: CharacterWithIndex,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl BackupManagerPopup {
    #[must_use]
    pub fn new(character: CharacterWithIndex, selected_index: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(selected_index));
        Self {
            character,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: BackupManagerPopupCommand) {
        self.commands
            .push(PopupCommand::BackupManager(self.character.1, command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: BackupManagerPopupCommand) {
        self.push_command(command);
        self.close = true;
    }

    /// Get the backup at a specified index from the source character.
    #[inline]
    #[must_use]
    pub fn get_backup(&self, index: usize) -> Option<&crate::wow::WowBackup> {
        self.character.0.backups().get(index)
    }
}

impl Popup for BackupManagerPopup {
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
            KeyCode::Char('e' | 'E') => {
                if let Some(selected) = self.state.selected()
                    && self.character.0.backups().len() > selected
                {
                    self.push_command(BackupManagerPopupCommand::ToggleBackupPin(selected));
                }
            }
            KeyCode::Char('d' | 'D') => {
                if let Some(selected) = self.state.selected()
                    && let Some(backup) = self.get_backup(selected).cloned()
                {
                    let command = PopupCommand::BackupManager(
                        self.character.1,
                        BackupManagerPopupCommand::DeleteBackup(selected),
                    );
                    self.commands.push(command.with_confirm_and_line(vec![
                        Span::from("Delete `"),
                        backup.formatted_name().bold(),
                        Span::from("`"),
                    ]));
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(
                Line::from(vec![
                    Span::from(" Backups for "),
                    self.character.0.display_span(true),
                    Span::from(" "),
                ])
                .bold(),
            )
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .bg(STD_BG)
            .padding(Padding::symmetric(1, 0));

        let items = self
            .character
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
            .fg(STD_FG)
            .highlight_style(Style::new().bold().bg(HOVER_BG))
            .direction(ListDirection::TopToBottom);

        list_with_scrollbar(list_view, area, buf, &mut self.state);
    }

    fn process_message(&mut self, message: &PopupMessage) {
        match message {
            PopupMessage::UpdateCharacter(updated_char) => {
                if updated_char.0.is_same_character(&self.character.0) {
                    self.character = updated_char.clone();
                    log::debug!("Updated backup manager popup character info");
                }
            }
        }
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "backup_manager_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        let selected_backup_index = self.state.selected().unwrap_or(0);
        let pin_backup_opt = if let Some(backup) = self.get_backup(selected_backup_index)
            && backup.is_pinned
        {
            "E: Unpin Backup"
        } else {
            "E: Pin Backup"
        };
        Some(vec![
            "↑/↓",
            "Esc: Close",
            "D: Delete Backup",
            pin_backup_opt,
        ])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        Some(&mut self.commands)
    }
    fn popup_min_width(&self) -> u16 {
        64
    }
    fn popup_min_height(&self) -> u16 {
        16
    }
}
