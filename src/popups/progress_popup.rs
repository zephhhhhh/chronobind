use std::fmt::Debug;

#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::{
    backend::task::{BackendTaskPtr, IOTask},
    ui::messages::AppMessage,
    widgets::popup::{Popup, popup_block},
};

use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Style,
    widgets::{Gauge, ListState, Widget},
};

/// Popup for paste confirmation.
#[derive(Debug)]
pub struct ProgressPopup {
    /// Backend task being tracked by the popup.
    pub task: BackendTaskPtr,

    /// Whether the popup should close.
    pub close: bool,

    /// Commands issued by the popup.
    pub commands: Vec<AppMessage>,
}

impl ProgressPopup {
    #[must_use]
    pub fn new(task: IOTask) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut popup = Self {
            task: Box::new(task),
            close: false,
            commands: vec![],
        };

        popup.run_task();

        popup
    }

    /// Start running the task.
    /// If it fails to start, log an error and close the popup.
    fn run_task(&mut self) {
        if !self.task.run() {
            log::error!("Failed to start task `{}`", self.task.task_name());
            self.close();
        }
    }

    /// Check if the task has finalised and handle closure and errors.
    fn check_finalise(&mut self) {
        if self.task.finished() {
            if let Some(error) = self.task.error() {
                // self.commands
                //     .push(AppMessage::ShowError("Task Error".to_string(), error));
                log::error!("Task error: `{error}`");
                self.close();
                return;
            }

            if let Some(after_msg) = self.task.after_messages() {
                self.commands.extend_from_slice(&after_msg);
            }
            if let Some(next) = self.task.next_task() {
                self.task = next;
                self.run_task();
            } else {
                self.close();
            }
        }
    }
}

impl Popup for ProgressPopup {
    #[allow(
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        self.task.poll();

        let render_area = area.inner(Margin::new(1, 1));
        let block = popup_block(format!(" {} ", self.task.task_name()))
            .border_style(Style::default().fg(LOG_INFO_FG));

        let progress_label = self.task.progress_formatted(true);
        let percentage = self.task.progress_ui();

        let progress_bar = Gauge::default()
            .block(block)
            .gauge_style(Style::new().fg(STD_FG).bg(HOVER_BG))
            .label(progress_label)
            .percent(percentage.clamp(0, 100));

        self.check_finalise();

        Widget::render(progress_bar, render_area, buf);
    }

    fn should_close(&self) -> bool {
        self.close
    }
    fn close(&mut self) {
        self.close = true;
    }
    fn popup_identifier(&self) -> &'static str {
        "progress_popup"
    }
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        None
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
        60
    }
}
