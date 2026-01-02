#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    widgets::popup::{Popup, PopupCommand},
    wow,
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::Line,
    widgets::{
        Block, Clear, List, ListDirection, ListItem, ListState, Padding, StatefulWidget, Widget,
    },
};

/// Different commands that can be issued from a branch popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BranchPopupCommand {
    SelectBranch(String),
}

/// Popup for branch options for a character.
#[derive(Debug, Clone)]
pub struct BranchPopup {
    /// The available branches.
    pub branches: Vec<wow::WowInstall>,
    /// The currently selected branch.
    pub current_branch: Option<String>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl BranchPopup {
    #[must_use]
    pub fn new(branches: Vec<wow::WowInstall>, current_branch: Option<String>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            branches,
            current_branch,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: BranchPopupCommand) {
        self.commands.push(PopupCommand::Branch(command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: BranchPopupCommand) {
        self.push_command(command);
        self.close = true;
    }
}

impl Popup for BranchPopup {
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
                    && selected < self.branches.len()
                {
                    self.push_command_close(BranchPopupCommand::SelectBranch(
                        self.branches[selected].branch_ident.clone(),
                    ));
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
            .title(Line::styled(" Select a WoW Branch ", title_style))
            .border_set(border::ROUNDED)
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black))
            .padding(Padding::symmetric(1, 0));

        let items = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let hovered = i == self.state.selected().unwrap_or(0);
                let content = if let Some(selected_branch) = &self.current_branch
                    && item.branch_ident == *selected_branch
                {
                    format!("{} (current)", item.display_branch_name())
                } else {
                    item.display_branch_name()
                };
                let line = Line::from(dual_highlight_str(content, hovered)).centered();
                ListItem::new(line)
            })
            .collect::<Vec<ListItem>>();

        let list_view = List::new(items)
            .block(block)
            .style(Style::new().white())
            .highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected)
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
        "branch_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        Some(vec!["↑/↓", "↵/Space: Select", "Esc: Close"])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        Some(&mut self.commands)
    }
}
