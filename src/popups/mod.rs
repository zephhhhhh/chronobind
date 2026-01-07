use std::fmt::Display;

use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::palette::{
    PALETTE, SCROLL_DOWN_ICON, SCROLL_UP_ICON, checkbox, highlight_str, highlight_symbol,
    highlight_symbol_rev,
};

pub mod backup_manager_popup;
pub mod backup_popup;
pub mod branch_popup;
pub mod confirm_popup;
pub mod export_manager_popup;
pub mod options_popup;
pub mod progress_popup;
pub mod restore_popup;

/// Create a line representing a toggle option.
#[inline]
fn toggle_option(title: &str, selected: bool, hovered: bool) -> Line<'_> {
    let colour = PALETTE.selection_fg(selected);
    let content = format!("{} {}", checkbox(selected), highlight_str(title, hovered));
    Line::from(content).fg(colour)
}

/// Create a dual highlighted symbol for hovered items, for lines with multiple spans.
#[inline]
fn wrap_selection(mut spans: Vec<Span>, hovered: bool) -> Line {
    if hovered {
        spans.insert(0, Span::from(highlight_symbol(hovered)));
        spans.push(Span::from(highlight_symbol_rev(hovered)));
    }
    Line::from(spans).centered()
}

/// Create a dual highlighted symbol for hovered items, for a provided line
#[inline]
fn wrap_selection_text(mut line: Text<'static>, hovered: bool) -> Text<'static> {
    for line in &mut line.lines {
        line.spans.insert(0, Span::from(highlight_symbol(hovered)));
        line.spans.push(Span::from(highlight_symbol_rev(hovered)));
    }
    line
}

/// Render a widget with an optional scrollbar if the content length exceeds the viewable area.
#[inline]
pub fn with_optional_scrollbar<T: StatefulWidget>(
    widget: T,
    area: Rect,
    buf: &mut Buffer,
    state: &mut T::State,
    content_length: usize,
    offset: usize,
) {
    StatefulWidget::render(widget, area, buf, state);
    if content_length > 1 {
        let mut scrollbar_state = ScrollbarState::new(content_length).position(offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(SCROLL_UP_ICON))
            .end_symbol(Some(SCROLL_DOWN_ICON));

        StatefulWidget::render(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut scrollbar_state,
        );
    }
}

/// Render a list with an optional scrollbar if the content length exceeds the viewable area.
#[inline]
pub fn list_with_scrollbar(list: List<'_>, area: Rect, buf: &mut Buffer, state: &mut ListState) {
    let height = area.height.saturating_sub(3);
    let offset = state.offset();
    let content_length = list.len().saturating_sub(height as usize);

    with_optional_scrollbar(list, area, buf, state, content_length, offset);
}

/// Format an option for display purposes.
#[inline]
#[must_use]
pub fn format_option<T: Display>(opt: Option<&T>) -> String {
    opt.map_or_else(|| "None".to_string(), std::string::ToString::to_string)
}
