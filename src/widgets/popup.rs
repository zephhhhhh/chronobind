use std::fmt::Debug;

use crate::ChronoBindApp;

pub type PopupHandlerFn = dyn Fn(&mut ChronoBindApp, usize, &str, usize) -> bool + Send + Sync;

/// State for a popup menu.
pub struct PopupState {
    pub title: String,
    pub items: Vec<String>,
    pub selected_index: usize,
    pub context_id: Option<usize>,
    pub handler: Option<Box<PopupHandlerFn>>,
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for PopupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopupState")
            .field("title", &self.title)
            .field("items", &self.items)
            .field("selected_index", &self.selected_index)
            .field("context_id", &self.context_id)
            .finish()
    }
}

impl PopupState {
    pub fn new(title: impl Into<String>, items: Vec<String>, handler: Box<PopupHandlerFn>) -> Self {
        Self {
            title: title.into(),
            items,
            selected_index: 0,
            context_id: None,
            handler: Some(handler),
        }
    }

    #[must_use]
    pub const fn with_context(mut self, context_id: usize) -> Self {
        self.context_id = Some(context_id);
        self
    }

    pub const fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub const fn move_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn get_selected(&self) -> Option<&str> {
        self.items
            .get(self.selected_index)
            .map(std::string::String::as_str)
    }

    pub fn handle_selection(&self, app: &mut ChronoBindApp) -> Option<bool> {
        let selected = self.get_selected()?;
        let context_id = self.context_id?;
        let handler = self.handler.as_ref()?;
        Some(handler(app, self.selected_index, selected, context_id))
    }
}
