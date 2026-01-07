#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    ChronoBindAppConfig,
    popups::toggle_option,
    ui::{KeyCodeExt, messages::AppMessage},
    widgets::popup::{Popup, popup_block, popup_list_no_block},
    wow::WoWInstalls,
};

use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{ListState, StatefulWidget, Widget},
};

/// Different commands that can be issued from a restore popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionsPopupCommand {
    /// Command to update the app configuration with new settings.
    UpdateConfiguration(ChronoBindAppConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionKind {
    ShowFriendlyNames,
    MockMode,
    MaximumAutoBackups,
    PreferredBranch,
}

impl OptionKind {
    /// Get the list of available options.
    #[must_use]
    fn get_options_list() -> Vec<Self> {
        vec![
            Self::ShowFriendlyNames,
            Self::MockMode,
            Self::MaximumAutoBackups,
            Self::PreferredBranch,
        ]
    }

    /// Get the display title for the option.
    #[inline]
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::ShowFriendlyNames => "Show friendly file names",
            Self::MockMode => "Mock mode (Don't perform file operations)",
            Self::MaximumAutoBackups => "Maximum allowed automatic backups",
            Self::PreferredBranch => "Preferred WoW branch",
        }
    }

    /// Generate the display line for the option.
    #[inline]
    #[must_use]
    pub fn get_line(
        &self,
        config: &ChronoBindAppConfig,
        installs: &WoWInstalls,
        hovered: bool,
    ) -> Line<'static> {
        match self {
            Self::ShowFriendlyNames => {
                toggle_option(self.title(), config.show_friendly_names, hovered)
            }
            Self::MockMode => toggle_option(self.title(), config.mock_mode, hovered),
            Self::MaximumAutoBackups => {
                let displayed_text = config.maximum_auto_backups.map_or_else(
                    || UNLIMITED_SYMBOL.to_string(),
                    |max_backups| format!("{max_backups}"),
                );
                Line::from(highlight_str(
                    format!("{}: {displayed_text}", self.title()),
                    hovered,
                ))
            }
            Self::PreferredBranch => {
                let displayed_text =
                    preferred_branch_display(config.preferred_branch.as_ref(), installs);
                Line::from(highlight_str(
                    format!("{}: {displayed_text}", self.title()),
                    hovered,
                ))
            }
        }
    }

    /// Generate the bottom bar segments for the hovered option.
    #[inline]
    #[must_use]
    pub fn get_bottom_bar_segments(&self) -> Vec<String> {
        match self {
            Self::ShowFriendlyNames | Self::MockMode => {
                vec![format!("{ENTER_SYMBOL}/→/Space: Toggle")]
            }
            Self::MaximumAutoBackups | Self::PreferredBranch => {
                vec!["←/→: Adjust".to_string()]
            }
        }
    }
}

/// Popup for configuring options of `ChronoBind`.
#[derive(Debug, Clone)]
pub struct OptionsPopup {
    /// The current application configuration.
    pub configuration: ChronoBindAppConfig,
    /// The currently selected `WoW` branch.
    pub selected_branch: Option<String>,
    /// Detected `WoW` branches.
    pub branches: WoWInstalls,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl OptionsPopup {
    #[must_use]
    pub fn new(
        config: ChronoBindAppConfig,
        branches: WoWInstalls,
        selected_branch: Option<String>,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            configuration: config,
            selected_branch,
            branches,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Push a command to the popup's command list.
    #[inline]
    pub fn push_command(&mut self, command: OptionsPopupCommand) {
        self.commands.push(AppMessage::Options(command));
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
    /// Interact with a specific option.
    fn interact_with_option(&mut self, option: &OptionKind) {
        let mut config_changed = false;
        match option {
            OptionKind::ShowFriendlyNames => {
                self.configuration.show_friendly_names = !self.configuration.show_friendly_names;
                config_changed = true;
            }
            OptionKind::MockMode => {
                self.configuration.mock_mode = !self.configuration.mock_mode;
                config_changed = true;
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
    /// Draw the options menu within the popup.
    fn draw_options_menu(&mut self, area: Rect, buf: &mut Buffer) {
        let selected_idx = self.selected_index();
        let options = OptionKind::get_options_list();
        let items = options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                option.get_line(&self.configuration, &self.branches, i == selected_idx)
            })
            .collect::<Vec<Line>>();

        let list_view = popup_list_no_block(items);
        StatefulWidget::render(list_view, area, buf, &mut self.state);
    }

    /// Draw the dedication credits at the bottom of the popup.
    fn draw_credits(area: Rect, buf: &mut Buffer) {
        const MY_LOVE: &str = "Larissa";

        let line = Line::from(vec![
            Span::from("Dedicated to "),
            Span::from(MY_LOVE).bold().italic().fg(PALETTE.heart_fg),
            Span::from(" <3").fg(PALETTE.heart_fg),
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

    /// Get the currently selected option in the options menu.
    fn selected_option(&self) -> Option<OptionKind> {
        let options = OptionKind::get_options_list();
        options.get(self.selected_index()).cloned()
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
            let next_branch_ident = self.branches.installs[next_index].branch_ident.clone();
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
            let previous_branch_ident = self.branches.installs[previous_index].branch_ident.clone();
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
}

impl Popup for OptionsPopup {
    fn on_key_down(&mut self, key: &KeyEvent) {
        let list = OptionKind::get_options_list();
        let selected_opt = list.get(self.selected_index());
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.state.select_previous();
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.state.select_next();
            }
            KeyCode::Left | KeyCode::Char('a') => match selected_opt {
                Some(OptionKind::PreferredBranch) => {
                    if !self.branches.is_empty() && self.select_previous_branch() {
                        self.push_update_command();
                    }
                }
                Some(OptionKind::MaximumAutoBackups) => {
                    if self.decrement_max_auto_backups() {
                        self.push_update_command();
                    }
                }
                _ => {}
            },
            KeyCode::Right | KeyCode::Char('d') => match selected_opt {
                Some(OptionKind::PreferredBranch) => {
                    if !self.branches.is_empty() && self.select_next_branch() {
                        self.push_update_command();
                    }
                }
                Some(OptionKind::MaximumAutoBackups) => {
                    if self.increment_max_auto_backups() {
                        self.push_update_command();
                    }
                }
                Some(opt) => self.interact_with_option(opt),
                _ => {}
            },
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(o) = selected_opt {
                    self.interact_with_option(o);
                }
            }
            KeyCode::Esc | KeyCode::Char('q' | 'o') => {
                self.close();
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, frame: &mut Frame<'_>) {
        let block = popup_block(" ChronoBind Options ");
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(block.inner(area));

        Widget::render(block, area, frame.buffer_mut());
        self.draw_options_menu(chunks[0], frame.buffer_mut());
        Self::draw_credits(chunks[1], frame.buffer_mut());
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
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        let mut options = vec!["↑/↓".to_string()];
        if let Some(option) = self.selected_option() {
            options.extend_from_slice(&option.get_bottom_bar_segments());
        }
        options.push("Esc: Close".to_string());

        Some(options)
    }
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        Some(&mut self.commands)
    }
    fn popup_width_percent(&self) -> u16 {
        80
    }
    fn popup_height_percent(&self) -> u16 {
        80
    }
}

/// Get the display text for the preferred branch.
fn preferred_branch_display(preferred_branch: Option<&String>, installs: &WoWInstalls) -> String {
    let Some(branch_ident) = preferred_branch else {
        return "None".to_string();
    };
    let Some(install) = installs.find_branch(branch_ident) else {
        return format!("Unknown({branch_ident})");
    };
    install.display_branch_name()
}
