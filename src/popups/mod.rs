use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use crate::palette::{SCROLL_DOWN_ICON, SCROLL_UP_ICON, highlight_symbol, highlight_symbol_rev};

pub mod backup_manager_popup;
pub mod backup_popup;
pub mod branch_popup;
pub mod options_popup;
pub mod paste_popup;
pub mod restore_popup;

/// Create a dual highlighted symbol for hovered items, for lines with multiple spans.
fn wrap_selection(mut spans: Vec<Span>, hovered: bool) -> Line {
    if hovered {
        spans.insert(0, Span::from(highlight_symbol(hovered)));
        spans.push(Span::from(highlight_symbol_rev(hovered)));
    }
    Line::from(spans).centered()
}

/// Render a widget with an optional scrollbar if the content length exceeds the viewable area.
pub fn with_optional_scrollbar<T: StatefulWidget>(
    widget: T,
    area: Rect,
    buf: &mut Buffer,
    state: &mut T::State,
    content_length: usize,
    offset: usize,
) {
    StatefulWidget::render(widget, area, buf, state);
    if content_length > 0 {
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
pub fn list_with_scrollbar(list: List<'_>, area: Rect, buf: &mut Buffer, state: &mut ListState) {
    let height = area.height.saturating_sub(3);
    let offset = state.offset();
    let content_length = list.len().saturating_sub(height as usize);

    with_optional_scrollbar(list, area, buf, state, content_length, offset);
}
