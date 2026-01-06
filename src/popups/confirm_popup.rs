#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    popups::wrap_selection_text,
    ui::{
        KeyCodeExt,
        messages::{AppMessage, ConfirmActionText},
    },
    widgets::popup::{Popup, popup_block, popup_list},
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Margin, Rect},
    style::Style,
    text::Line,
    widgets::{Clear, ListItem, ListState, StatefulWidget, Widget},
};

/// Identifier for the confirmation popup.
pub const CONFIRM_POPUP_ID: &str = "confirm_popup";

/// Popup for paste confirmation.
#[derive(Debug, Clone)]
pub struct ConfirmationPopup {
    /// The command to perform after confirmation.
    pub action: AppMessage,
    /// An optional action line to display additional information.
    pub action_line: Option<ConfirmActionText>,

    /// Whether the popup should close.
    pub close: bool,
    /// The state of the list within the popup.
    pub state: ListState,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl ConfirmationPopup {
    /// Index of Cancel option.
    const CANCEL_IDX: usize = 0;
    /// Index of Confirm option.
    const CONFIRM_IDX: usize = 1;

    #[must_use]
    pub fn new(action: AppMessage, action_line: Option<ConfirmActionText>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            action,
            action_line,

            close: false,
            state: list_state,

            commands: vec![],
        }
    }

    /// Confirm the action and queue the action.
    #[inline]
    pub fn confirmed(&mut self) {
        self.commands.push(self.action.clone());
        log::debug!("ConfirmationPopup: Action confirmed: {:?}", self.action);
        self.close = true;
    }
}

impl Popup for ConfirmationPopup {
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
                    if selected == Self::CONFIRM_IDX {
                        self.confirmed();
                    } else {
                        self.close = true;
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.close = true;
            }
            _ => {}
        }
    }

    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let render_area = area.inner(Margin::new(1, 1));

        let block =
            popup_block(" Are you sure? ").border_style(Style::default().fg(PALETTE.log_warn_fg));

        let selected_idx = self.state.selected().unwrap_or(0);
        let items = [
            {
                let content = dual_highlight_str("Cancel", selected_idx == Self::CANCEL_IDX);
                ListItem::new(Line::from(content).centered())
            },
            self.action_line.as_ref().map_or_else(
                || {
                    let content = dual_highlight_str("Confirm", selected_idx == Self::CONFIRM_IDX);
                    ListItem::new(Line::from(content).centered())
                },
                |action_line| {
                    // TODO: I really want this to wrap to the next line if needed >:(
                    let content = wrap_selection_text(
                        action_line.to_text(),
                        selected_idx == Self::CONFIRM_IDX,
                    );
                    ListItem::new(content.centered())
                },
            ),
        ];

        let list_view = popup_list(block, items);

        Widget::render(Clear, render_area, buf);
        StatefulWidget::render(list_view, render_area, buf, &mut self.state);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        CONFIRM_POPUP_ID
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

    fn popup_height_percent(&self) -> u16 {
        0
    }
    #[allow(clippy::cast_possible_truncation)]
    fn popup_min_height(&self) -> u16 {
        6
    }

    #[allow(clippy::cast_possible_truncation)]
    fn popup_min_width(&self) -> u16 {
        self.action_line.as_ref().map_or(50, |action_line| {
            (action_line.to_text().width() + 10) as u16
        }) + 2
    }
}

// pub fn wrap_text_ratatui(input: &str, width: u16) -> Vec<Line<'static>> {
//     if width == 0 {
//         return Vec::new();
//     }

//     let text = Text::from(input);

//     let config = ReflowConfig {
//         wrap: Wrap { trim: false },
//         max_width: width as usize,
//     };

//     reflow_text(&text, &config)
//         .into_iter()
//         .map(Line::into_owned)
//         .collect()
// }
