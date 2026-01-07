use std::fmt::Debug;

use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyEvent, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, List, ListDirection, ListItem, Padding, Widget};

use crate::palette::PALETTE;
pub use crate::ui::messages::{AppMessage, PopupMessage};

/// Type alias for a boxed `Popup` trait object.
pub type PopupPtr = Box<dyn Popup + Send + Sync>;

impl Debug for Box<dyn Popup + Send + Sync> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup").finish()
    }
}

/// Trait representing a popup widget.
pub trait Popup {
    /// Called when a key is pressed down.
    fn on_key_down(&mut self, _key: &KeyEvent) {}
    /// Draw the popup into the given area of the buffer.
    fn draw(&mut self, area: Rect, frame: &mut Frame<'_>);
    /// Determine if the popup should close.
    fn should_close(&self) -> bool;

    /// Close the popup.
    fn close(&mut self);

    /// Get the name of the popup.
    fn popup_identifier(&self) -> &'static str;

    /// Get options for the bottom bar, if any.
    fn bottom_bar_options(&self) -> Option<Vec<String>> {
        None
    }

    /// Get mutable reference to internal commands, if any.
    fn internal_commands_mut(&mut self) -> Option<&mut Vec<AppMessage>> {
        None
    }

    // Default implementations..

    /// Handle any events for the popup.
    /// Returns true if the popup handled the event. (I.e. block further processing.)
    fn handle_event(&mut self, event: &Event) -> bool {
        if let Event::Key(key_event) = event
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
        self.draw(popup_area, frame);
    }
    /// Retrieve and clear any commands issued by the popup.
    fn commands(&mut self) -> Option<Vec<AppMessage>> {
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

// Helper functions..

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

// Popup styling functions..

/// Standard padding for popups.
pub const POPUP_PADDING: Padding = Padding::symmetric(1, 0);
/// Standard title alignment for popups.
pub const POPUP_TITLE_ALIGNMENT: Alignment = Alignment::Center;
/// Standard border style for popups.
pub const POPUP_BORDER_STYLE: border::Set<'static> = border::ROUNDED;

/// Create a consistently styled popup block with a title for use in popups, does not stylise the inner content,
/// if you want a consistent inner styling, use `popup_block` instead.
#[inline]
pub fn popup_block_raw<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::bordered()
        .title(title.into().bold())
        .title_alignment(POPUP_TITLE_ALIGNMENT)
        .border_set(POPUP_BORDER_STYLE)
        .bg(PALETTE.std_bg)
}

/// Create a consistently styled popup block with a title for use in popups, does style and center the inner content,
/// if you do not want this, use `popup_block_raw` instead.
#[inline]
pub fn popup_block<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    popup_block_raw(title).padding(POPUP_PADDING)
}

// List styling functions..

/// Standard list direction for popup lists.
pub const POPUP_LIST_DIRECTION: ListDirection = ListDirection::TopToBottom;

/// Create a consistently styled list for use in popups, without a block.
#[inline]
pub fn popup_list_no_block<'a, T>(items: T) -> List<'a>
where
    T: IntoIterator,
    T::Item: Into<ListItem<'a>>,
{
    List::new(items)
        .fg(PALETTE.std_bg)
        .highlight_style(Style::new().bold().bg(PALETTE.hover_bg))
        .direction(POPUP_LIST_DIRECTION)
}

/// Create a consistently styled list for use in popups.
#[inline]
pub fn popup_list<'a, T>(block: Block<'a>, items: T) -> List<'a>
where
    T: IntoIterator,
    T::Item: Into<ListItem<'a>>,
{
    popup_list_no_block(items).block(block)
}
