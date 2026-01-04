#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    CharacterWithIndex,
    popups::wrap_selection,
    ui::{KeyCodeExt, messages::AppMessage},
    widgets::popup::{Popup, popup_block, popup_list},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    text::{Line, Span},
    widgets::{ListItem, ListState, StatefulWidget},
};

/// Different commands that can be issued from a backup popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BackupPopupCommand {
    /// Command to open the manage backups popup.
    ManageBackups,
    /// Command to backup selected files.
    BackupSelectedFiles,
    /// Command to backup all files.
    BackupAllFiles,
    /// Command to restore from backup.
    RestoreFromBackup,
    /// Command to restore from copied character's backups.
    RestoreFromCopiedBackups,
}

/// Popup for backup options for a character.
#[derive(Debug, Clone)]
pub struct BackupPopup {
    /// The character associated with the backup popup.
    pub character: CharacterWithIndex,
    /// The copied character if applicable, for restoring from their backups.
    pub copied_character: Option<CharacterWithIndex>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl BackupPopup {
    #[must_use]
    pub fn new(
        character: CharacterWithIndex,
        copied_character: Option<CharacterWithIndex>,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            character,
            copied_character,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: BackupPopupCommand) {
        self.commands
            .push(AppMessage::Backup(self.character.1, command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: BackupPopupCommand) {
        self.push_command(command);
        self.close = true;
    }
}

impl BackupPopup {
    pub const MANAGE_BACKUPS_IDX: usize = 0;
    pub const BACKUP_SELECTED_IDX: usize = 1;
    pub const BACKUP_ALL_IDX: usize = 2;
    pub const RESTORE_FROM_BACKUP_IDX: usize = 3;
    pub const RESTORE_FROM_COPIED_IDX: usize = 4;
}

impl Popup for BackupPopup {
    fn on_key_down(&mut self, key: &KeyEvent) {
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selected) = self.state.selected() {
                    match selected {
                        Self::MANAGE_BACKUPS_IDX => {
                            self.push_command(BackupPopupCommand::ManageBackups);
                        }
                        Self::BACKUP_SELECTED_IDX => {
                            self.push_command_close(BackupPopupCommand::BackupSelectedFiles);
                        }
                        Self::BACKUP_ALL_IDX => {
                            self.push_command_close(BackupPopupCommand::BackupAllFiles);
                        }
                        Self::RESTORE_FROM_BACKUP_IDX => {
                            self.push_command(BackupPopupCommand::RestoreFromBackup);
                        }
                        Self::RESTORE_FROM_COPIED_IDX => {
                            if self.copied_character.is_some() {
                                self.push_command(BackupPopupCommand::RestoreFromCopiedBackups);
                            } else {
                                log::warn!("No copied character to restore from.");
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'b') => {
                self.close();
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let block = popup_block(" Backup Options ");
        let item_names = [
            "Manage backups",
            "Backup selected files",
            "Backup all files",
            "Restore from backup",
        ];

        let selected_index = self.state.selected().unwrap_or(0);
        let mut items = item_names
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = dual_highlight_str(item, selected_index == i);
                let line = Line::from(content).centered();
                ListItem::new(line)
            })
            .collect::<Vec<ListItem>>();

        if let Some(copied_char) = &self.copied_character {
            let content = vec![
                Span::from("Restore from "),
                copied_char.0.display_span(true),
                Span::from("'s backups"),
            ];
            let line = wrap_selection(content, selected_index == item_names.len());
            items.push(ListItem::new(line));
        }

        let list_view = popup_list(block, items);

        StatefulWidget::render(list_view, area, buf, &mut self.state);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "backup_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        Some(vec!["↑/↓", "↵/Space: Select", "Esc: Close"])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        Some(&mut self.commands)
    }
    fn popup_min_width(&self) -> u16 {
        64
    }
}
