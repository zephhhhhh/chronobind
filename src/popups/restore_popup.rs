#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    Character,
    widgets::popup::{Popup, PopupCommand},
};

use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, List, ListDirection, ListItem, ListState, StatefulWidget, Widget},
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
    pub character: Character,
    /// The index of the character in the main character list.
    pub character_index: usize,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl RestorePopup {
    #[must_use]
    pub fn new(character: Character, index: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            character,
            character_index: index,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: RestorePopupCommand) {
        self.commands
            .push(PopupCommand::Restore(self.character_index, command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: RestorePopupCommand) {
        self.push_command(command);
        self.close = true;
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
                    && selected < self.character.character.backups.len()
                {
                    self.push_command_close(RestorePopupCommand::RestoreBackup(selected));
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        // Get title styling based on context (character if applicable)
        let title = format!(" Restore {} ", self.character.display_name(true));

        let title_style = Style::default().add_modifier(Modifier::BOLD).fg(self
            .character
            .character
            .class
            .class_colour());

        let block = Block::bordered()
            .title(Line::styled(title, title_style))
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        let items = self
            .character
            .character
            .backups
            .iter()
            .enumerate()
            .map(|(i, backup)| {
                let hovered = i == self.state.selected().unwrap_or(0);
                let content = format!(
                    "{} {}{}",
                    backup.char_name,
                    display_backup_time(&backup.timestamp),
                    if backup.is_paste { " (Pasted)" } else { "" }
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

        Widget::render(Clear, area, buf);
        StatefulWidget::render(list_view, area, buf, &mut self.state);
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
        Some(vec!["↑/↓: Nav", "↵/Space: Select", "Esc: Close"])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        Some(&mut self.commands)
    }
}
