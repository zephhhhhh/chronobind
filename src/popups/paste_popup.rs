#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    Character,
    widgets::popup::{Popup, PopupCommand},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListDirection, ListItem, ListState, StatefulWidget, Widget},
};

/// Different commands that can be issued from a backup popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PastePopupCommand {
    /// Command to confirm pasting files.
    ConfirmPaste,
}

/// Popup for paste confirmation.
#[derive(Debug, Clone)]
pub struct PasteConfirmPopup {
    /// The character associated with the backup popup.
    pub character: Character,
    /// The index of the character in the main character list.
    pub character_index: usize,
    /// File count to be pasted.
    pub file_count: usize,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl PasteConfirmPopup {
    /// Index of Cancel option.
    const CANCEL_IDX: usize = 0;
    /// Index of Confirm Paste option.
    const CONFIRM_IDX: usize = 1;

    #[must_use]
    pub fn new(character: Character, index: usize, file_count: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            character,
            character_index: index,
            file_count,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: PastePopupCommand) {
        self.commands
            .push(PopupCommand::Paste(self.character_index, command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: PastePopupCommand) {
        self.push_command(command);
        self.close = true;
    }
}

impl Popup for PasteConfirmPopup {
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
                if let Some(selected) = self.state.selected() {
                    if selected == Self::CONFIRM_IDX {
                        self.push_command_close(PastePopupCommand::ConfirmPaste);
                    } else {
                        self.close = true;
                    }
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
        let title_style = Style::default().add_modifier(Modifier::BOLD);

        let block = Block::bordered()
            .title(Line::styled(" Are you sure? ", title_style))
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        let selected_idx = self.state.selected().unwrap_or(0);
        let plural = if self.file_count == 1 { "" } else { "s" };
        let items = [
            {
                let content = dual_highlight_symbol("Cancel", selected_idx == Self::CANCEL_IDX);
                let line = Line::from(content).centered();
                ListItem::new(line)
            },
            {
                let prompt = Span::from(format!("Paste {} file{} to ", self.file_count, plural));
                let char_name = Span::styled(
                    self.character.display_name(true),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(self.character.class_colour()),
                );
                ListItem::new(wrap_selection(
                    vec![prompt, char_name],
                    selected_idx == Self::CONFIRM_IDX,
                ))
            },
        ];

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

    fn popup_min_width(&self) -> u16 {
        50
    }
}

/// Create a dual highlighted symbol for hovered items, for lines with multiple spans.
fn wrap_selection(mut spans: Vec<Span>, hovered: bool) -> Line {
    if hovered {
        spans.insert(0, Span::from(highlight_symbol(hovered)));
        spans.push(Span::from(highlight_symbol_rev(hovered)));
    }
    Line::from(spans).centered()
}
