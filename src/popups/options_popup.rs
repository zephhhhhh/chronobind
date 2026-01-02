#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    ChronoBindAppConfig,
    widgets::popup::{Popup, PopupCommand},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, List, ListDirection, ListState, Padding, StatefulWidget, Widget},
};

/// Different commands that can be issued from a restore popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionsPopupCommand {
    /// Command to update the app configuration with new settings.
    UpdateConfiguration(ChronoBindAppConfig),
}

/// Popup for configuring options of `ChronoBind`.
#[derive(Debug, Clone)]
pub struct OptionsPopup {
    pub configuration: ChronoBindAppConfig,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl OptionsPopup {
    #[must_use]
    pub fn new(config: ChronoBindAppConfig) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            configuration: config,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: OptionsPopupCommand) {
        self.commands.push(PopupCommand::Options(command));
    }

    /// Push a command to the popup's command list and close the popup.
    #[inline]
    pub fn push_command_close(&mut self, command: OptionsPopupCommand) {
        self.push_command(command);
        self.close = true;
    }
}

impl OptionsPopup {
    /// Index of Show Friendly Names option.
    pub const SHOW_FRIENDLY_NAMES_IDX: usize = 0;
    /// Index of Mock Mode option.
    pub const MOCK_MODE_IDX: usize = 1;
    /// Index of Preferred Branch option.
    pub const PREFERRED_BRANCH_IDX: usize = 2;

    fn interact_with_option(&mut self, index: usize) {
        let mut config_changed = false;
        match index {
            Self::SHOW_FRIENDLY_NAMES_IDX => {
                self.configuration.show_friendly_names = !self.configuration.show_friendly_names;
                config_changed = true;
            }
            Self::MOCK_MODE_IDX => {
                self.configuration.mock_mode = !self.configuration.mock_mode;
                config_changed = true;
            }
            Self::PREFERRED_BRANCH_IDX => {
                log::warn!("Preferred branch selection not implemented yet");
            }
            _ => {}
        }

        if config_changed {
            self.push_command(OptionsPopupCommand::UpdateConfiguration(
                self.configuration.clone(),
            ));
        }
    }

    /// Create a line representing a toggle option.
    fn toggle_option(title: &str, selected: bool, hovered: bool) -> Line<'_> {
        let symbol = if selected { "✓" } else { " " };
        let colour = if selected { SELECTED_FG } else { STD_FG };
        let content = format!("[{}] {}", symbol, highlight_str(title, hovered));
        Line::from(content).style(Style::default().fg(colour))
    }
}

impl Popup for OptionsPopup {
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
                    self.interact_with_option(selected);
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'Q' | 'o' | 'O') => {
                self.close();
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let title_style = Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White);
        let block = Block::bordered()
            .title(Line::styled(" ChronoBind Options ", title_style))
            .border_set(border::ROUNDED)
            .padding(Padding::symmetric(1, 0))
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        let selected_idx = self.state.selected().unwrap_or(0);
        let items = [
            Self::toggle_option(
                "Show friendly file names",
                self.configuration.show_friendly_names,
                selected_idx == Self::SHOW_FRIENDLY_NAMES_IDX,
            ),
            Self::toggle_option(
                "Mock mode (no actual file operations)",
                self.configuration.mock_mode,
                selected_idx == Self::MOCK_MODE_IDX,
            ),
            Line::from(highlight_str(
                "Preferred WoW branch",
                selected_idx == Self::PREFERRED_BRANCH_IDX,
            )),
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
        self.push_command_close(OptionsPopupCommand::UpdateConfiguration(
            self.configuration.clone(),
        ));
    }
    fn popup_identifier(&self) -> &'static str {
        "options_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        Some(vec!["↑/↓", "↵/Space: Select", "Esc: Close"])
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        Some(&mut self.commands)
    }
    fn popup_width_percent(&self) -> u16 {
        80
    }
    fn popup_height_percent(&self) -> u16 {
        80
    }
}
