use std::path::PathBuf;

use ratatui::{
    style::{Color, Stylize},
    text::Span,
};

use crate::wow::{
    SAVED_VARIABLES_DIR, WoWCharacter, WoWCharacterBackup, WoWCharacterFile, WoWInstall,
};

/// Type alias for a character index.
pub type CharacterIndex = usize;

/// Representation of a `WoW` character along with its selected files and
/// options inside the app UI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(clippy::struct_field_names)]
pub struct Character {
    /// The underlying `WoW` character data.
    pub character: WoWCharacter,
    /// Whether the addon options section is collapsed.
    pub addon_options_collapsed: bool,

    /// Which config files are selected.
    selected_config_files: Vec<bool>,
    /// Which addon files are selected.
    selected_addon_files: Vec<bool>,
}

impl Character {
    #[must_use]
    pub fn new(character: &WoWCharacter) -> Self {
        let config_file_count = character.config_files.len();
        let addon_file_count = character.addon_files.len();
        Self {
            character: character.clone(),
            selected_config_files: vec![false; config_file_count],
            selected_addon_files: vec![false; addon_file_count],
            addon_options_collapsed: false,
        }
    }
}

// Accessors..
impl Character {
    /// Get the display name of the character, optionally including the realm.
    #[must_use]
    pub fn display_name(&self, show_realm: bool) -> String {
        if show_realm {
            format!("{} - {}", self.name(), self.realm())
        } else {
            self.name().to_string()
        }
    }

    /// Get the realm of the character.
    #[inline]
    #[must_use]
    pub fn realm(&self) -> &str {
        &self.character.realm
    }

    /// Get the name of the character.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.character.name
    }

    /// Get the branch of the character.
    #[inline]
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.character.branch
    }

    /// Get the account of the character.
    #[inline]
    #[must_use]
    pub fn account(&self) -> &str {
        &self.character.account
    }

    /// Get the class colour of the character.
    #[inline]
    #[must_use]
    pub const fn class_colour(&self) -> Color {
        self.character.class.class_colour()
    }

    /// Get the config files of the character.
    #[inline]
    #[must_use]
    pub fn config_files(&self) -> &[WoWCharacterFile] {
        &self.character.config_files
    }

    /// Get the addon files of the character.
    #[inline]
    #[must_use]
    pub fn addon_files(&self) -> &[WoWCharacterFile] {
        &self.character.addon_files
    }

    /// Get the backups of the character.
    #[inline]
    #[must_use]
    pub fn backups(&self) -> &[WoWCharacterBackup] {
        &self.character.backups
    }

    /// Returns `true` if the other character represents the same character
    /// (same `name`, `realm`, `account`, and `branch`).
    #[inline]
    #[must_use]
    pub fn is_same_character(&self, other: &Self) -> bool {
        self.character.is_same_character(&other.character)
    }
}

impl Character {
    /// Check if a config file at the given index is selected.
    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_config_file_selected(&self, index: usize) -> bool {
        self.selected_config_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Toggle the selected status of a config file at the given index.
    #[inline]
    pub fn toggle_config_file_selected(&mut self, index: usize) -> bool {
        self.selected_config_files
            .get_mut(index)
            .is_some_and(|selected| {
                *selected = !*selected;
                *selected
            })
    }

    /// Check if an addon file at the given index is selected.
    #[inline]
    #[must_use]
    #[allow(dead_code)]
    pub fn is_addon_file_selected(&self, index: usize) -> bool {
        self.selected_addon_files
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Toggle the selected status of an addon file at the given index.
    #[inline]
    pub fn toggle_addon_file_selected(&mut self, index: usize) -> bool {
        self.selected_addon_files
            .get_mut(index)
            .is_some_and(|selected| {
                *selected = !*selected;
                *selected
            })
    }

    /// Check if any config files are selected.
    #[inline]
    #[must_use]
    pub fn any_config_file_selected(&self) -> bool {
        self.selected_config_files.iter().any(|&s| s)
    }

    /// Check if any addon file is selected.
    #[inline]
    #[must_use]
    pub fn any_addon_file_selected(&self) -> bool {
        self.selected_addon_files.iter().any(|&s| s)
    }

    /// Check if any files (regular or addon) are selected.
    #[inline]
    #[must_use]
    pub fn any_file_selected(&self) -> bool {
        self.any_config_file_selected() || self.any_addon_file_selected()
    }

    /// Check if all config files are selected.
    #[inline]
    #[must_use]
    pub fn all_config_files_selected(&self) -> bool {
        self.selected_config_files.iter().all(|&s| s)
    }

    /// Check if all addon files are selected.
    #[inline]
    #[must_use]
    pub fn all_addon_files_selected(&self) -> bool {
        self.selected_addon_files.iter().all(|&s| s)
    }

    /// Set the selected status of all config files.
    #[inline]
    pub fn set_all_config_selected(&mut self, state: bool) {
        self.selected_config_files.fill(state);
    }

    /// Set the selected status of all addon files.
    #[inline]
    pub fn set_all_addon_selected(&mut self, state: bool) {
        self.selected_addon_files.fill(state);
    }

    /// Set the selected status of all files (config and addon).
    #[inline]
    pub fn set_all_selected(&mut self, state: bool) {
        self.set_all_config_selected(state);
        self.set_all_addon_selected(state);
    }

    /// Get the count of selected config files.
    #[inline]
    #[must_use]
    pub fn selected_config_count(&self) -> usize {
        self.selected_config_files.iter().filter(|&&s| s).count()
    }

    /// Get the count of selected addon files.
    #[inline]
    #[must_use]
    pub fn selected_addon_count(&self) -> usize {
        self.selected_addon_files.iter().filter(|&&s| s).count()
    }

    /// Get the total count of selected files (config and addon).
    #[inline]
    #[must_use]
    pub fn total_selected_count(&self) -> usize {
        self.selected_config_count() + self.selected_addon_count()
    }

    /// Get all selected files from both config files and addon files.
    #[must_use]
    pub fn get_all_selected_files(&self) -> Vec<PathBuf> {
        let mut selected_paths = Vec::new();

        for (i, selected) in self.selected_config_files.iter().enumerate() {
            if *selected && let Some(file) = self.config_files().get(i) {
                selected_paths.push(file.get_full_filename().into());
            }
        }

        for (i, selected) in self.selected_addon_files.iter().enumerate() {
            if *selected && let Some(file) = self.addon_files().get(i) {
                let path = PathBuf::from(SAVED_VARIABLES_DIR).join(file.get_full_filename());
                selected_paths.push(path);
            }
        }

        selected_paths
    }
}

// UI helper functions..
impl Character {
    /// Get a styled span for the character's display name, using the appropriate class colour.
    #[inline]
    #[must_use]
    pub fn display_span(&self, show_realm: bool) -> Span<'static> {
        let content = self.display_name(show_realm);
        Span::from(content).fg(self.class_colour())
    }
}

// Character with meta data..

/// Representation of a `WoW` character along with its selected files and
/// options inside the app UI, and it's associated index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(clippy::struct_field_names)]
pub struct CharacterWithIndex(pub Character, pub usize);

impl AsRef<Character> for CharacterWithIndex {
    fn as_ref(&self) -> &Character {
        &self.0
    }
}

impl AsMut<Character> for CharacterWithIndex {
    fn as_mut(&mut self) -> &mut Character {
        &mut self.0
    }
}

/// Type alias for a character with its associated `WoW` installation.
pub type CharacterWithInstall<'a> = (&'a Character, &'a WoWInstall);
