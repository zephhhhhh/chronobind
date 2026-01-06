use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Padding, Paragraph, Widget, Wrap};

#[allow(clippy::wildcard_imports)]
use crate::palette::*;
use crate::tui_log;
use crate::ui::KeyCodeExt;

/// Widget responsible for displaying and controlling the console output panel.
#[derive(Debug, Default)]
pub struct ConsoleWidget {
    /// Whether the console output is visible.
    show: bool,
    /// Current scroll offset (newest at bottom; positive values scroll upward).
    pub scroll_offset: usize,
}

impl ConsoleWidget {
    /// Create a new console widget.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show: false,
            scroll_offset: 0,
        }
    }

    /// Toggle visibility of the console panel.
    pub const fn toggle_show(&mut self) {
        self.show = !self.show;
    }

    /// Set visibility explicitly.
    pub const fn set_show(&mut self, show: bool) {
        self.show = show;
    }

    /// Check if the console is visible.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.show
    }

    /// Handle key input when the console panel is active.
    pub fn handle_input(&mut self, key: &KeyEvent) {
        const SPEED_MULTIPLIER: usize = 3;

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let speed_multiplier = if ctrl { SPEED_MULTIPLIER } else { 1 };
        match key.keycode_lower() {
            KeyCode::Up | KeyCode::Char('w') => {
                self.scroll_offset = self.scroll_offset.saturating_add(speed_multiplier);
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(speed_multiplier);
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(10 * speed_multiplier);
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10 * speed_multiplier);
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                self.scroll_offset = tui_log::TuiLogger::MAX_LOG_SIZE;
            }
            _ => {}
        }
    }

    /// Render the console output panel.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Console Output ").bold();
        let block = Block::bordered()
            .title(title)
            .border_set(ratatui::symbols::border::THICK)
            .padding(Padding::symmetric(1, 0));

        let log_lines: Option<Vec<Line>> = tui_log::with_debug_logs(|logs| {
            let visible_lines = area.height.saturating_sub(2) as usize;
            let total_logs = logs.len();

            let max_scroll = total_logs.saturating_sub(visible_lines);
            self.scroll_offset = self.scroll_offset.min(max_scroll);

            logs.iter()
                .rev()
                .skip(max_scroll.saturating_sub(self.scroll_offset))
                .take(visible_lines)
                .map(|log| {
                    let color = PALETTE.log_level_colour(log.level());
                    Line::from(log.content().to_string()).fg(color)
                })
                .collect()
        });

        let log_text = log_lines.unwrap_or_else(|| {
            vec![Line::from("Failed to retrieve logs").fg(PALETTE.log_error_fg)]
        });

        Paragraph::new(log_text)
            .wrap(Wrap { trim: false })
            .block(block)
            .render(area, buf);
    }
}
