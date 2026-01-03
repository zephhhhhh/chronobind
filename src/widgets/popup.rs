use std::fmt::Debug;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::{Clear, Widget};

/// Type alias for a boxed `Popup` trait object.
pub type PopupPtr = Box<dyn Popup + Send + Sync>;

impl Debug for Box<dyn Popup + Send + Sync> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup").finish()
    }
}

/// Type alias for commands that can be issued from a popup.
pub type PopupCommand = crate::PopupAppCommand;
/// Type alias for commands that can be issued to a popup from the main app.
pub type PopupMessage = crate::AppPopupMessage;

/// Trait representing a popup widget.
pub trait Popup {
    /// Called when a key is pressed down.
    fn on_key_down(&mut self, _key: &KeyEvent) {}
    /// Draw the popup into the given area of the buffer.
    fn draw(&mut self, area: Rect, buf: &mut Buffer);
    /// Determine if the popup should close.
    fn should_close(&self) -> bool;

    /// Close the popup.
    fn close(&mut self);

    /// Get the name of the popup.
    fn popup_identifier(&self) -> &'static str;

    /// Get options for the bottom bar, if any.
    fn bottom_bar_options(&self) -> Option<Vec<&str>> {
        None
    }

    /// Get mutable reference to internal commands, if any.
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<PopupCommand>> {
        None
    }

    // Default implementations..

    /// Handle any events for the popup.
    /// Returns true if the popup handled the event. (I.e. block further processing.)
    fn handle_event(&mut self, key: &Event) -> bool {
        if let Event::Key(key_event) = key
            && key_event.kind == KeyEventKind::Press
        {
            self.on_key_down(key_event);
        }
        true
    }
    /// Render the popup.
    fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let popup_area = popup_area(
            area,
            self.popup_width_percent(),
            self.popup_height_percent(),
            self.popup_min_width(),
            self.popup_min_height(),
        );

        Widget::render(Clear, popup_area, frame.buffer_mut());
        self.draw(popup_area, frame.buffer_mut());
    }
    /// Retrieve and clear any commands issued by the popup.
    fn commands(&mut self) -> Option<Vec<PopupCommand>> {
        self.internal_commands_mut().and_then(|cmds| {
            if cmds.is_empty() {
                None
            } else {
                Some(std::mem::take(cmds))
            }
        })
    }

    /// Process a message sent to the popup.
    fn process_message(&mut self, _message: &PopupMessage) {}

    /// Get the width percentage for the popup.
    #[inline]
    #[must_use]
    fn popup_width_percent(&self) -> u16 {
        35
    }
    /// Get the height percentage for the popup.
    #[inline]
    #[must_use]
    fn popup_height_percent(&self) -> u16 {
        30
    }
    /// Get the minimum width for the popup.
    #[inline]
    #[must_use]
    fn popup_min_width(&self) -> u16 {
        30
    }
    /// Get the minimum height for the popup.
    #[inline]
    #[must_use]
    fn popup_min_height(&self) -> u16 {
        10
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
/// with minimum width and height constraints.
#[inline]
#[must_use]
pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16, min_x: u16, min_y: u16) -> Rect {
    let width = ((area.width * percent_x) / 100).max(min_x);
    let height = ((area.height * percent_y) / 100).max(min_y);

    let vertical = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
