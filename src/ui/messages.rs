// Popup -> App communication..

use ratatui::text::{Line, Span, Text};

use crate::{
    popups::{
        backup_manager_popup::BackupManagerPopupCommand, backup_popup::BackupPopupCommand,
        branch_popup::BranchPopupCommand, options_popup::OptionsPopupCommand,
        restore_popup::RestorePopupCommand,
    },
    ui::character::{CharacterIndex, CharacterWithIndex},
};

/// A message from a popup to the main application.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppMessage {
    /// Commands from the backup popup.
    Backup(CharacterIndex, BackupPopupCommand),
    /// Commands from the restore from backup popup.
    Restore(CharacterIndex, RestorePopupCommand),
    /// Paste from the copied character to the provided target character.
    Paste(CharacterIndex),
    /// Commands from the branch selection popup.
    Branch(BranchPopupCommand),
    /// Commands from the options popup.
    Options(OptionsPopupCommand),
    /// Commands from the backup manager popup.
    BackupManager(CharacterIndex, BackupManagerPopupCommand),
    /// Generic confirm action.
    /// Opens a confirmation popup for the given action.
    ConfirmAction(Box<Self>, Option<ConfirmActionText>),
}

impl AppMessage {
    /// Wrap the message in a confirmation action, requiring the user to confirm the action before proceeding.
    #[inline]
    #[must_use]
    pub fn with_confirm(self) -> Self {
        Self::ConfirmAction(Box::new(self), None)
    }

    /// Wrap the command in a confirmation action, and a custom line to display as the confirm action.
    #[inline]
    #[must_use]
    pub fn with_confirm_and_line(self, action_line: impl Into<ConfirmActionText>) -> Self {
        Self::ConfirmAction(Box::new(self), Some(action_line.into()))
    }
}

// App to Popup communication..

/// A message from the main application to a popup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PopupMessage {
    /// Command to update the characters data for the popup.
    UpdateCharacter(CharacterWithIndex),
}

// Confirm action text wrapper.

/// Wrapper for text to display in a confirm action popup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfirmActionText(Text<'static>);

impl ConfirmActionText {
    /// Get the inner text.
    #[inline]
    #[must_use]
    pub fn to_text(&self) -> Text<'static> {
        self.0.clone()
    }
}

impl From<Text<'static>> for ConfirmActionText {
    fn from(text: Text<'static>) -> Self {
        Self(text)
    }
}
impl From<Line<'static>> for ConfirmActionText {
    fn from(line: Line<'static>) -> Self {
        Self(Text::from(line))
    }
}
impl From<Span<'static>> for ConfirmActionText {
    fn from(span: Span<'static>) -> Self {
        Self(Text::from(span))
    }
}
impl From<Vec<Span<'static>>> for ConfirmActionText {
    fn from(spans: Vec<Span<'static>>) -> Self {
        Self(Text::from(Line::from(spans)))
    }
}
