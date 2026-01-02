#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    ChronoBindAppConfig,
    widgets::popup::{Popup, PopupCommand},
    wow,
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, List, ListDirection, ListState, Padding, StatefulWidget, Widget},
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
    /// The current application configuration.
    pub configuration: ChronoBindAppConfig,
    /// Detected `WoW` branches.
    pub branches: Vec<wow::WowInstall>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<PopupCommand>,
}

impl OptionsPopup {
    #[must_use]
    pub fn new(config: ChronoBindAppConfig, branches: Vec<wow::WowInstall>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            configuration: config,
            branches,

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

    /// Push an update configuration command to the popup's command list.
    #[inline]
    pub fn push_update_command(&mut self) {
        self.push_command(OptionsPopupCommand::UpdateConfiguration(
            self.configuration.clone(),
        ));
    }
}

impl OptionsPopup {
    /// Index of Show Friendly Names option.
    pub const SHOW_FRIENDLY_NAMES_IDX: usize = 0;
    /// Index of Mock Mode option.
    pub const MOCK_MODE_IDX: usize = 1;
    /// Index of Maximum Auto Backups option.
    pub const MAX_AUTO_BACKUPS_IDX: usize = 2;
    /// Index of Preferred Branch option.
    pub const PREFERRED_BRANCH_IDX: usize = 3;

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
            Self::MAX_AUTO_BACKUPS_IDX => {
                log::warn!("Maximum auto backups adjustment not implemented yet");
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
}

impl OptionsPopup {
    /// Create a line representing a toggle option.
    fn toggle_option(title: &str, selected: bool, hovered: bool) -> Line<'_> {
        let colour = if selected { SELECTED_FG } else { STD_FG };
        let content = format!("{} {}", checkbox(selected), highlight_str(title, hovered));
        Line::from(content).style(Style::default().fg(colour))
    }

    /// Create a line representing the maximum automatic backups option.
    fn get_maximum_auto_backups_option_line(
        max_backups: Option<usize>,
        hovered: bool,
    ) -> Line<'static> {
        let displayed_text = max_backups.map_or_else(
            || UNLIMITED_SYMBOL.to_string(),
            |max_backups| format!("{max_backups}"),
        );
        Line::from(highlight_str(
            format!("Maximum allowed automatic backups: {displayed_text}"),
            hovered,
        ))
    }

    /// Create a line representing the maximum automatic backups option.
    fn get_preferred_branch_text(&self, hovered: bool) -> String {
        let Some(branch_ident) = &self.configuration.preferred_branch else {
            return "Preferred WoW branch: None".to_string();
        };
        let Some(install) = self.find_wow_branch(branch_ident) else {
            return format!("Preferred WoW branch: Unknown({branch_ident})");
        };
        highlight_str(
            format!("Preferred WoW branch: {}", install.display_branch_name()),
            hovered,
        )
    }

    /// Draw the options menu within the popup.
    fn draw_options_menu(&mut self, area: Rect, buf: &mut Buffer) {
        let selected_idx = self.selected_index();
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
            Self::get_maximum_auto_backups_option_line(
                self.configuration.maximum_auto_backups,
                selected_idx == Self::MAX_AUTO_BACKUPS_IDX,
            ),
            Line::from(self.get_preferred_branch_text(selected_idx == Self::PREFERRED_BRANCH_IDX)),
        ];

        let list_view = List::new(items)
            .style(Style::new().white())
            .highlight_style(Style::new().add_modifier(Modifier::BOLD).bg(HOVER_BG))
            .direction(ListDirection::TopToBottom);

        StatefulWidget::render(list_view, area, buf, &mut self.state);
    }

    /// Draw the dedication credits at the bottom of the popup.
    fn draw_credits(area: Rect, buf: &mut Buffer) {
        const MY_LOVE: &str = "Larissa";
        const HEART_FG: Color = Color::Rgb(186, 117, 170);

        let line = Line::from(vec![
            Span::from("Dedicated to "),
            Span::from(MY_LOVE)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC)
                .fg(HEART_FG),
            Span::from(" <3").fg(HEART_FG),
        ])
        .right_aligned();

        Widget::render(line, area, buf);
    }
}

impl OptionsPopup {
    /// Get the currently selected index in the options menu.
    fn selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    /// Decrement the maximum automatic backups setting.
    /// Returns `true` if the setting was changed.
    const fn decrement_max_auto_backups(&mut self) -> bool {
        if let Some(current) = self.configuration.maximum_auto_backups {
            if current > 0 {
                self.configuration.maximum_auto_backups = Some(current - 1);
            } else {
                self.configuration.maximum_auto_backups = None;
            }
            return true;
        }
        false
    }

    /// Increment the maximum automatic backups setting.
    /// Returns `true` if the setting was changed.
    const fn increment_max_auto_backups(&mut self) -> bool {
        if let Some(current) = self.configuration.maximum_auto_backups {
            self.configuration.maximum_auto_backups = Some(current + 1);
        } else {
            self.configuration.maximum_auto_backups = Some(0);
        }
        true
    }

    /// Select the next `WoW` branch in the list.
    fn select_next_branch(&mut self) -> bool {
        if let Some(current_index) = self.get_selected_branch_index() {
            let next_index = (current_index + 1) % self.branches.len();
            let next_branch_ident = self.branches[next_index].branch_ident.clone();
            self.configuration.preferred_branch = Some(next_branch_ident);
            return true;
        }
        false
    }

    /// Select the previous `WoW` branch in the list.
    fn select_previous_branch(&mut self) -> bool {
        if let Some(current_index) = self.get_selected_branch_index() {
            let previous_index = if current_index == 0 {
                self.branches.len() - 1
            } else {
                current_index - 1
            };
            let previous_branch_ident = self.branches[previous_index].branch_ident.clone();
            self.configuration.preferred_branch = Some(previous_branch_ident);
            return true;
        }
        false
    }

    /// Find the selected index of the preferred `WoW` branch.
    #[inline]
    #[must_use]
    pub fn get_selected_branch_index(&self) -> Option<usize> {
        let selected_ident = self.configuration.preferred_branch.clone()?;
        let (i, _) = self.branches.iter().enumerate().find(|(_, install)| {
            install.branch_ident.to_lowercase() == selected_ident.to_lowercase()
        })?;
        Some(i)
    }

    /// Find a `WoW` installation by its branch identifier.
    #[inline]
    #[must_use]
    pub fn find_wow_branch(&self, branch: &str) -> Option<&wow::WowInstall> {
        self.branches
            .iter()
            .find(|install| install.branch_ident.to_lowercase() == branch.to_lowercase())
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
            KeyCode::Left | KeyCode::Char('a' | 'A') => {
                let selected_index = self.selected_index();
                match selected_index {
                    Self::PREFERRED_BRANCH_IDX => {
                        if !self.branches.is_empty() && self.select_previous_branch() {
                            self.push_update_command();
                        }
                    }
                    Self::MAX_AUTO_BACKUPS_IDX => {
                        if self.decrement_max_auto_backups() {
                            self.push_update_command();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Right | KeyCode::Char('d' | 'D') => {
                let selected_index = self.selected_index();
                match selected_index {
                    Self::PREFERRED_BRANCH_IDX => {
                        if !self.branches.is_empty() && self.select_next_branch() {
                            self.push_update_command();
                        }
                    }
                    Self::MAX_AUTO_BACKUPS_IDX => {
                        if self.increment_max_auto_backups() {
                            self.push_update_command();
                        }
                    }
                    _ => {}
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(block.inner(area));

        Widget::render(block, area, buf);
        self.draw_options_menu(chunks[0], buf);
        Self::draw_credits(chunks[1], buf);
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
        let mut options = vec!["↑/↓"];
        let selected_index = self.state.selected().unwrap_or(0);
        if selected_index == Self::MAX_AUTO_BACKUPS_IDX
            || selected_index == Self::PREFERRED_BRANCH_IDX
        {
            options.push("←/→: Adjust");
        } else {
            options.push("↵/Space: Toggle");
        }
        options.push("Esc: Close");
        Some(options)
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
