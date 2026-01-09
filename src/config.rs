use std::path::PathBuf;

use directories::ProjectDirs;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{
    files::{AnyResult, ensure_directory},
    wow,
};

/// Application configuration options.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChronoBindAppConfig {
    /// Whether to show friendly names for files instead of raw filenames.
    pub show_friendly_names: bool,
    /// Whether to operate in mock mode (no actual file operations).
    pub mock_mode: bool,
    /// Maximum automatic backups to keep per character.
    pub maximum_auto_backups: Option<usize>,
    /// Whether to display character levels in the UI, if available.
    pub display_character_levels: bool,
    /// Preferred branch.
    pub preferred_branch: Option<String>,
}

impl ChronoBindAppConfig {
    /// Default maximum automatic backups to keep per character.
    pub const DEFAULT_MAXIMUM_AUTO_BACKUPS: usize = 10;
}

impl Default for ChronoBindAppConfig {
    fn default() -> Self {
        //let mock_mode = cfg!(debug_assertions);
        let mock_mode = true;
        Self {
            show_friendly_names: true,
            mock_mode,
            preferred_branch: Some(wow::WOW_RETAIL_IDENT.to_string()),
            display_character_levels: true,
            maximum_auto_backups: Some(Self::DEFAULT_MAXIMUM_AUTO_BACKUPS),
        }
    }
}

impl ChronoBindAppConfig {
    /// Default configuration file name.
    pub const CONFIG_FILE_NAME: &str = "chronobind.config";

    /// Load configuration from the configuration file directory.
    /// # Errors
    /// Errors if reading or parsing the configuration file fails.
    pub fn load_config() -> AnyResult<Option<Self>> {
        let config_file_path = get_config_dir().join(Self::CONFIG_FILE_NAME);
        log::debug!(
            "Loading configuration from `{}`",
            config_file_path.display()
        );

        if !config_file_path.exists() {
            return Ok(None);
        }

        let config_src_str = std::fs::read_to_string(&config_file_path)?;
        log::debug!("Successfully read configuration file.. Parsing..");

        let parsed = ron::from_str::<Self>(&config_src_str)?;
        log::info!("Successfully parsed configuration file");

        Ok(Some(parsed))
    }

    /// Load configuration from the configuration file directory, or return default if not found.
    /// # Errors
    /// Errors if reading or parsing the configuration file fails.
    pub fn load_config_or_default() -> AnyResult<Self> {
        let config = Self::load_config()?;
        config.map_or_else(
            || {
                log::debug!("No configuration file found; using default configuration");
                Ok(Self::default())
            },
            |cfg| {
                log::debug!("Using loaded configuration");
                Ok(cfg)
            },
        )
    }

    /// Save configuration to a RON file.
    /// # Errors
    /// Errors if writing the configuration file fails.
    pub fn save_to_file(&self) -> AnyResult<()> {
        let config_dir = get_config_dir();
        ensure_directory(&config_dir, false)?;
        let config_file_path = config_dir.join(Self::CONFIG_FILE_NAME);

        log::debug!(
            "Preparing to save configuration to `{}`",
            config_file_path.display()
        );

        let config_data = ron::ser::to_string_pretty(self, PrettyConfig::default())?;

        log::debug!("Saving configuration to `{}`", config_file_path.display());
        std::fs::write(config_file_path, config_data)?;
        log::debug!("Successfully saved configuration file");

        Ok(())
    }
}

/// Project qualifier for application directories.
const PROJ_QUALIFIER: &str = "dev";
/// Project organisation for application directories.
const PROJ_ORGANISATION: &str = "zephhhhhh";
/// Project organisation for application directories.
const PROJ_APPLICATION: &str = "chronobind";

/// Get the project directories for `ChronoBind`.
/// # Panics
/// Panics if the project directories cannot be determined.
fn get_project_dirs() -> ProjectDirs {
    ProjectDirs::from(PROJ_QUALIFIER, PROJ_ORGANISATION, PROJ_APPLICATION)
        .expect("Failed to determine project directories")
}

/// Get the configuration file path for `ChronoBind`.
/// # Panics
/// Panics if the project directories cannot be determined.
fn get_config_dir() -> PathBuf {
    let proj_dirs = get_project_dirs();
    proj_dirs.config_dir().to_path_buf()
}
