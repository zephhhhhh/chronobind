#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    ui::{KeyCodeExt, messages::AppMessage},
    widgets::popup::{Popup, popup_block, popup_list},
    wow::WowInstall,
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::Rect,
    text::Line,
    widgets::{ListItem, ListState, StatefulWidget},
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
    pub branches: Vec<WowInstall>,
    /// The currently selected branch.
    pub current_branch: Option<String>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl BranchPopup {
    #[must_use]
    pub fn new(branches: Vec<WowInstall>, current_branch: Option<String>) -> Self {
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
        self.commands.push(AppMessage::Branch(command));
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
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
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
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let block = popup_block(" Select a WoW Branch ");

        let selected_index = self.state.selected().unwrap_or(0);
        let items = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = if let Some(selected_branch) = &self.current_branch
                    && item.branch_ident == *selected_branch
                {
                    format!("{} (current)", item.display_branch_name())
                } else {
                    item.display_branch_name()
                };
                let line = Line::from(dual_highlight_str(content, i == selected_index)).centered();
                ListItem::new(line)
            })
            .collect::<Vec<ListItem>>();

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
        "branch_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        Some(vec![
            "↑/↓".to_string(),
            format!("{}/Space: Select", ENTER_SYMBOL),
            "Esc: Close".to_string(),
        ])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        Some(&mut self.commands)
    }
}
