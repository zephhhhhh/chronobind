use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use const_format::concatcp;
use itertools::Itertools;
use prost::Message;
use ratatui::style::Color;

use crate::{backend::BACKUP_FILE_EXTENSION, files::read_folders_to_string};

mod productdb {
    include!(concat!(env!("OUT_DIR"), "/productdb.rs"));
}

// Locating WoW installs..
// TODO: Platform independence..
/// Path to the Battle.net Agent product database file.
const BNET_AGENT_PRODUCT_DB_PATH: &str = "C:\\ProgramData\\Battle.net\\Agent\\product.db";
/// Identifier prefix for World of Warcraft product codes.
const WOW_PRODUCT_CODE_IDENT: &str = "wow";
/// Prefix for World of Warcraft product codes with branch identifiers.
const WOW_PRODUCT_CODE_BRANCH_PREFIX: &str = concatcp!(WOW_PRODUCT_CODE_IDENT, "_");
/// Identifier for the retail branch of World of Warcraft.
pub const WOW_RETAIL_IDENT: &str = "retail";

/// Represents a World of Warcraft installation.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WoWInstall {
    /// The Battle.net product code for this installation.
    pub product_code: String,
    /// The branch identifier for this installation (e.g., "retail", "classic", etc.).
    pub branch_ident: String,
    /// The root installation path for this World of Warcraft installation.
    pub install_path: String,
}

impl WoWInstall {
    /// Returns true if this installation is the retail version of World of Warcraft.
    #[inline]
    #[must_use]
    pub fn is_retail(&self) -> bool {
        self.branch_ident == WOW_RETAIL_IDENT
    }

    /// Returns the product directory name for this installation.
    /// This is the name of the folder in the World of Warcraft installation directory,
    /// corresponding to the branch, I.e. retail is "_retail_".
    #[inline]
    #[must_use]
    pub fn get_product_dir(&self) -> String {
        format!("_{}_", self.branch_ident)
    }

    /// Returns the product directory name for this installation.
    /// This is the name of the folder in the World of Warcraft installation directory,
    /// corresponding to the branch, I.e. retail is "_retail_".
    #[inline]
    #[must_use]
    pub fn get_branch_path(&self) -> PathBuf {
        let install_path = PathBuf::from(&self.install_path);
        install_path.join(self.get_product_dir())
    }

    /// Returns the path to the `ChronoBind` directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_chronobind_dir(&self) -> PathBuf {
        let branch_path = self.get_branch_path();
        branch_path.join(CHRONOBIND_DIR)
    }

    /// Returns the path to the `ChronoBind` backups directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_character_backups_dir(&self) -> PathBuf {
        let chronobind_dir = self.get_chronobind_dir();
        chronobind_dir.join(CHARACTER_BACKUPS_DIR)
    }

    /// Returns a formatted version of the branch name for display purposes.
    #[inline]
    pub fn display_branch_name(&self) -> String {
        if self.is_retail() {
            "Retail".to_string()
        } else {
            self.branch_ident
                .split('_')
                .map(capitalise)
                .collect::<Vec<String>>()
                .join(" ")
        }
    }

    /// Joins a given path segment to the installation path, returning the result.
    /// (Does not modify the original installation path.)
    #[inline]
    #[must_use]
    pub fn install_path_join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let install_path = PathBuf::from(&self.install_path);
        install_path.join(path)
    }
}

/// Collection of located `WoW` installations.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WoWInstalls {
    /// Root path of `WoW` installations. (I.e. the folder that contains '_retail_', '_classic_', etc.)
    pub root_path: Option<PathBuf>,
    /// Located `WoW` installations.
    pub installs: Vec<WoWInstall>,
}

impl WoWInstalls {
    /// Create a new `WoWInstalls` instance from a vector of installs.
    /// Assumes the root path is the common parent directory of the first install.
    #[must_use]
    pub fn new_from_installs(installs: Vec<WoWInstall>) -> Self {
        let root_path = installs
            .first()
            .map(|install| PathBuf::from(&install.install_path));
        Self {
            root_path,
            installs,
        }
    }

    /// Get the number of located `WoW` installations.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.installs.len()
    }

    /// Returns `true` if there are no `WoW` installations located.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.installs.is_empty()
    }
}

impl WoWInstalls {
    /// Find a `WoW` installation by its branch identifier.
    #[inline]
    #[must_use]
    pub fn find_branch(&self, branch: &str) -> Option<&WoWInstall> {
        self.installs
            .iter()
            .find(|install| install.branch_ident.to_lowercase() == branch.to_lowercase())
    }

    /// Get an iterator over all located `WoW` installations.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &WoWInstall> {
        self.installs.iter()
    }
}

/// Extract World of Warcraft installation data from a Battle.net product installation entry.
fn extract_wow_install_data(product: &productdb::ProductInstall) -> Option<WoWInstall> {
    if !product.product_code.starts_with(WOW_PRODUCT_CODE_IDENT) {
        return None;
    }
    let branch_ident = if product.product_code == WOW_PRODUCT_CODE_IDENT {
        WOW_RETAIL_IDENT.to_string()
    } else {
        product
            .product_code
            .strip_prefix(WOW_PRODUCT_CODE_BRANCH_PREFIX)
            .or_else(|| product.product_code.strip_prefix(WOW_PRODUCT_CODE_IDENT))?
            .to_string()
    };
    Some(WoWInstall {
        product_code: product.product_code.clone(),
        branch_ident,
        install_path: product
            .settings
            .as_ref()
            .map(|settings| settings.install_path.clone())?,
    })
}

/// Locate all World of Warcraft installations on the system.
/// # Errors
/// This function will return an error if the Battle.net product database cannot be read or decoded.
pub fn locate_wow_installs() -> Result<Vec<WoWInstall>, Box<dyn std::error::Error>> {
    let product_db = get_product_db()?;

    Ok(product_db
        .product_install
        .iter()
        .filter_map(extract_wow_install_data)
        .collect())
}

/// Get the product database from the Battle.net agent 'product.db' file, used to find
/// the install location of World of Warcraft.
/// # Errors
/// This function will return an error if the 'product.db' file cannot be decoded.
fn get_product_db() -> Result<productdb::Database, Box<dyn std::error::Error>> {
    let product_db_bytes = std::fs::read(BNET_AGENT_PRODUCT_DB_PATH)?;
    Ok(productdb::Database::decode(product_db_bytes.as_slice())?)
}

/// Capitalises the first letter of the string
fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

/// Name of the user directory for `WoW` settings.
pub const USER_DIR: &str = "WTF";
/// Name of the user directory for `WoW` settings.
pub const INTERFACE_DIR: &str = "Interface";
/// Name of the account directory within the `WoW` user settings.
pub const ACCOUNT_DIR: &str = "Account";
/// Name of the `SavedVariables` directory within the `WoW` user settings.
pub const SAVED_VARIABLES_DIR: &str = "SavedVariables";

/// Check if a directory name is a valid `WoW` account directory name within the `WTF/Account` path.
#[inline]
fn is_account_dir(dir_name: &str) -> bool {
    dir_name != SAVED_VARIABLES_DIR && dir_name.chars().all(|c| c.is_numeric() || c == '#')
}

/// Finds all valid `WoW` account directories (not characters) in the given installation.
fn find_accounts_in_install(
    install: &WoWInstall,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let account_path = install.get_account_path();
    if account_path.is_dir() {
        // Filter all valid results from the directory,
        // ..then filter to only valid directories
        // ..then filter to only valid account directory names
        let accounts = read_folders_to_string(account_path)?
            .filter(|d| is_account_dir(d))
            .collect::<Vec<String>>();
        Ok(accounts)
    } else {
        Ok(vec![])
    }
}

impl WoWInstall {
    /// Returns the path to the user settings directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_wtf_path(&self) -> PathBuf {
        let install_path = self.get_branch_path();
        install_path.join(USER_DIR)
    }

    /// Returns the path to the interface directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_inteface_path(&self) -> PathBuf {
        let install_path = self.get_branch_path();
        install_path.join(INTERFACE_DIR)
    }

    /// Returns the path to the user accounts settings directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_account_path(&self) -> PathBuf {
        let install_path = self.get_wtf_path();
        install_path.join(ACCOUNT_DIR)
    }

    /// Returns the path to a specific account and realm directory within this installation.
    #[inline]
    #[must_use]
    pub fn get_realm_path(&self, account: &str, realm: &str) -> PathBuf {
        let account_path = self.get_account_path();
        account_path.join(account).join(realm)
    }

    /// Finds all valid `WoW` account directories (not characters) in this installation.
    #[inline]
    #[must_use]
    pub fn find_all_valid_accounts(&self) -> Option<Vec<String>> {
        find_accounts_in_install(self).ok()
    }

    /// Find all realms across all accounts in this installation.
    /// # Returns
    /// A vector of tuples containing `(account_name, realm_name)`.
    #[inline]
    #[must_use]
    pub fn find_all_realms(&self) -> Option<Vec<(String, String)>> {
        let accounts = self.find_all_valid_accounts()?;
        let x = accounts
            .iter()
            .flat_map(|account| {
                let account_path = self.get_account_path().join(account);
                read_folders_to_string(account_path).map_or_else(
                    |_| vec![],
                    |realms| {
                        realms
                            .filter(|d| d != SAVED_VARIABLES_DIR)
                            .map(|d| (account.clone(), d))
                            .collect::<Vec<(String, String)>>()
                    },
                )
            })
            .collect::<Vec<(String, String)>>();
        Some(x)
    }

    /// Find all realms characters across all realms across all accounts in this installation.
    #[inline]
    #[must_use]
    pub fn find_all_characters(&self) -> Option<Vec<WoWCharacter>> {
        let realms = self.find_all_realms()?;
        let characters = realms
            .iter()
            .flat_map(|(account_name, realm_name)| {
                let realm_path = self.get_realm_path(account_name, realm_name);
                read_folders_to_string(realm_path).map_or_else(
                    |_| vec![],
                    |chars| {
                        chars
                            .map(|char_name| WoWCharacter {
                                account: account_name.clone(),
                                branch: self.branch_ident.clone(),
                                name: char_name,
                                realm: realm_name.clone(),
                                class: WoWClass::Unknown,
                                config_files: vec![],
                                addon_files: vec![],
                                backups: vec![],
                            })
                            .collect::<Vec<WoWCharacter>>()
                    },
                )
            })
            .collect::<Vec<WoWCharacter>>();
        Some(characters)
    }

    /// Find all realms characters across all realms across all accounts in this installation.
    /// This populates all file information from the character directories as well.
    #[inline]
    #[must_use]
    pub fn find_all_characters_and_files(&self) -> Option<Vec<WoWCharacter>> {
        let mut chars = self.find_all_characters()?;
        for c in &mut chars {
            c.refresh_character_info(self);
        }
        Some(chars)
    }
}

/// Represents a file associated with a World of Warcraft character.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WoWCharacterFile {
    /// The full name of the character file, including its extension.
    pub name: String,
    /// The stem (filename without extension) of the character file.
    pub stem: String,
    /// The full path to the character file.
    pub path: PathBuf,
    /// An optional friendly name for the character file.
    pub friendly_name: Option<String>,
}

impl WoWCharacterFile {
    /// Returns the full filename of the character file, including its extension.
    #[inline]
    #[must_use]
    pub fn get_full_filename(&self) -> String {
        self.name.clone()
    }

    /// Returns file display name, using friendly name if available and requested.
    #[inline]
    #[must_use]
    pub fn display_name(&self, friendly: bool) -> String {
        if friendly && let Some(ref friendly_name) = self.friendly_name {
            return friendly_name.clone();
        }
        self.name.clone()
    }

    /// Returns file display stem, using friendly name if available and requested.
    #[inline]
    #[must_use]
    pub fn display_stem(&self, friendly: bool) -> String {
        if friendly && let Some(ref friendly_name) = self.friendly_name {
            return friendly_name.clone();
        }
        self.stem.clone()
    }

    /// Returns true if the file has a friendly name associated with it.
    #[inline]
    #[must_use]
    pub const fn has_friendly_name(&self) -> bool {
        self.friendly_name.is_some()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WoWCharacterBackup {
    /// The full path to the backup file.
    pub path: PathBuf,
    /// The name of the character associated with the backup.
    pub char_name: String,
    /// The timestamp of when the backup was created.
    pub timestamp: DateTime<Local>,
    /// Indicates whether the backup was created during a paste operation.
    pub is_paste: bool,
    /// Indicates whether the backup is pinned to not be auto-removed.
    pub is_pinned: bool,
}

impl WoWCharacterBackup {
    /// Returns a formatted string representation of the backup's name, including character name and timestamp.
    #[inline]
    #[must_use]
    pub fn formatted_name(&self) -> String {
        format!("{} | {}", self.char_name, self.formatted_timestamp())
    }

    /// Returns a formatted string representation of the backup's timestamp.
    #[inline]
    #[must_use]
    pub fn formatted_timestamp(&self) -> String {
        self.timestamp
            .format(crate::backend::DISPLAY_TIME_FORMAT)
            .to_string()
    }
}

/// Represents a World of Warcraft character.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WoWCharacter {
    /// The account name associated with the character.
    pub account: String,
    /// The branch identifier of the `WoW` installation the character belongs to.
    pub branch: String,
    /// The name of the character.
    pub name: String,
    /// The realm the character belongs to.
    pub realm: String,
    /// The class of the character.
    pub class: WoWClass,
    /// Files associated with the character's configuration.
    pub config_files: Vec<WoWCharacterFile>,
    /// Files associated with the character's addons.
    pub addon_files: Vec<WoWCharacterFile>,
    /// Backups associated with the character.
    pub backups: Vec<WoWCharacterBackup>,
}

/// Extensions for old (backup) config files.
const BACKUP_EXTENSIONS: [&str; 2] = ["bak", "old"];
/// Name of the main configuration file.
const CONFIG_WTF: &str = "config-cache.wtf";

/// Friendly names for common character files.
const FRIENDLY_NAMES: &[(&str, &str)] = &[
    ("bindings-cache.wtf", "Keybindings"),
    ("macros-cache.txt", "Macros"),
    ("cooldownmanager.txt", "Cooldown Manager"),
    ("layout-local.txt", "Legacy UI Layout"),
    ("edit-mode-cache-character.txt", "UI Layout"),
    ("AddOns.txt", "Enabled Addons"),
];

/// Name of the `ChronoBind` directory
pub const CHRONOBIND_DIR: &str = "ChronoBind";
/// Name of the `ChronoBind` directory
pub const CHARACTER_BACKUPS_DIR: &str = "Characters";
/// Name of the backups directory within a character's folder.
pub const BACKUPS_DIR_NAME: &str = "Backups";

/// Get a friendly name for a given filename, if available.
#[inline]
#[must_use]
fn get_friendly_name(filename: &str) -> Option<String> {
    FRIENDLY_NAMES
        .iter()
        .find(|(original_name, _)| filename == *original_name)
        .map(|(_, friendly_name)| friendly_name.to_string())
}

impl WoWCharacter {
    /// Returns the path to the character's directory.
    #[inline]
    #[must_use]
    pub fn get_character_path(&self, install: &WoWInstall) -> PathBuf {
        install
            .get_realm_path(&self.account, &self.realm)
            .join(&self.name)
    }

    /// Returns the path to the character's backups directory.
    #[inline]
    #[must_use]
    pub fn get_backups_dir(&self, install: &WoWInstall) -> PathBuf {
        let path_segments = [&self.account, &self.realm, &self.name];
        path_segments
            .into_iter()
            .fold(install.get_character_backups_dir(), |acc, p| acc.join(p))
    }

    /// Maps all files in the character's directory and addon directory.
    /// Also populates the character class information if possible.
    #[inline]
    #[allow(clippy::useless_let_if_seq)]
    pub fn refresh_character_info(&mut self, install: &WoWInstall) -> bool {
        let char_path = self.get_character_path(install);
        if !char_path.is_dir() || !char_path.exists() {
            return false;
        }

        let mut success = true;

        if !self.refresh_backups(install) {
            log::warn!(
                "Could not read backups for character {} on realm {} (account: {})",
                self.name,
                self.realm,
                self.account
            );
            success = false;
        }

        if !self.map_config_files(&char_path) {
            log::warn!(
                "Could not read files for character {} on realm {} (account: {})",
                self.name,
                self.realm,
                self.account
            );
            success = false;
        }

        if !self.map_addon_files(&char_path) {
            log::warn!(
                "Could not read addon files for character {} on realm {} (account: {})",
                self.name,
                self.realm,
                self.account
            );
            success = false;
        }

        if !self.try_to_load_class() {
            log::warn!(
                "Could not determine class for character {} on realm {} (account: {})",
                self.name,
                self.realm,
                self.account
            );
            success = false;
        }

        success
    }

    /// Maps all `WoW` character files in the character's directory.
    fn map_config_files(&mut self, char_path: &Path) -> bool {
        self.config_files = Vec::new();

        let Ok(files) = std::fs::read_dir(char_path) else {
            return false;
        };

        self.config_files = files
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().is_some_and(|ft| ft.is_file()))
            .filter_map(|entry| {
                let path = entry.path();
                let extension = path.extension()?.to_str()?.to_string();
                if BACKUP_EXTENSIONS.contains(&extension.to_lowercase().as_str()) {
                    return None;
                }

                let name = path.file_name()?.to_str()?.to_string();
                let stem = path.file_stem()?.to_str()?.to_string();

                Some(WoWCharacterFile {
                    name,
                    stem,
                    path,
                    friendly_name: get_friendly_name(&entry.file_name().to_string_lossy()),
                })
            })
            .sorted_by(|af, bf| bf.has_friendly_name().cmp(&af.has_friendly_name()))
            .collect();

        true
    }

    /// Maps all `WoW` addon files in the character's `SavedVariables` directory.
    fn map_addon_files(&mut self, char_path: &Path) -> bool {
        self.addon_files = Vec::new();

        let saved_variables_path = char_path.join(SAVED_VARIABLES_DIR);
        if !saved_variables_path.is_dir() || !saved_variables_path.exists() {
            return false;
        }
        let Ok(files) = std::fs::read_dir(&saved_variables_path) else {
            return false;
        };

        self.addon_files = files
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().is_some_and(|ft| ft.is_file()))
            .filter_map(|entry| {
                let path = entry.path();
                let extension = path.extension()?.to_str()?.to_string();

                if BACKUP_EXTENSIONS.contains(&extension.to_lowercase().as_str()) {
                    return None;
                }

                let name = path.file_name()?.to_str()?.to_string();
                let stem = path.file_stem()?.to_str()?.to_string();
                Some(WoWCharacterFile {
                    name,
                    stem,
                    path,
                    friendly_name: get_friendly_name(&entry.file_name().to_string_lossy()),
                })
            })
            .sorted_by(|af, bf| bf.has_friendly_name().cmp(&af.has_friendly_name()))
            .collect();

        true
    }

    /// Refresh the list of backups for this character.
    pub fn refresh_backups(&mut self, install: &WoWInstall) -> bool {
        self.backups = Vec::new();

        let backups_dir = self.get_backups_dir(install);
        if !backups_dir.is_dir() || !backups_dir.exists() {
            return false;
        }
        let Ok(files) = crate::files::read_files(&backups_dir) else {
            return false;
        };

        self.backups = files
            .map(|entry| entry.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|txt| txt.to_str().is_some_and(|txt| txt == BACKUP_FILE_EXTENSION))
            })
            .filter_map(|p| Some((p.clone(), p.file_stem()?.to_str()?.to_string())))
            .filter_map(|(p, stem)| {
                let (char_name, timestamp, is_paste, is_pinned) =
                    crate::backend::extract_backup_name(&stem)?;
                Some(WoWCharacterBackup {
                    char_name,
                    timestamp,
                    is_paste,
                    is_pinned,
                    path: p,
                })
            })
            .sorted_by(|a, b| {
                b.timestamp
                    .partial_cmp(&a.timestamp)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .collect();

        true
    }

    /// Attempts to load the character class from the config file.
    #[inline]
    fn try_to_load_class(&mut self) -> bool {
        let load_result = self
            .config_files
            .iter()
            .find(|f| f.get_full_filename().to_lowercase() == CONFIG_WTF)
            .and_then(|config_file| {
                let content = std::fs::read_to_string(&config_file.path).ok()?;
                let loot_class_line = content
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("set ejlootclass"))?;
                let parts: Vec<&str> = loot_class_line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let id = parts[2].trim_matches(|c| c == '\"').parse::<u8>().ok()?;
                    self.class = WoWClass::from_id(id);
                    Some(())
                } else {
                    None
                }
            });
        load_result.is_some()
    }
}

impl WoWCharacter {
    /// Returns `true` if the other character represents the same character
    /// (same `name`, `realm`, `account`, and `branch`).
    #[inline]
    #[must_use]
    pub fn is_same_character(&self, other: &Self) -> bool {
        self.name == other.name
            && self.realm == other.realm
            && self.account == other.account
            && self.branch == other.branch
    }

    /// Returns a vector of unpinned & automatically generated backups for the character.
    #[inline]
    #[must_use]
    pub fn unpinned_auto_backups(&self) -> Vec<WoWCharacterBackup> {
        self.backups
            .iter()
            .filter(|b| !b.is_pinned && b.is_paste)
            .cloned()
            .collect()
    }

    /// Returns the count of unpinned backups for the character.
    #[inline]
    #[must_use]
    pub fn unpinned_auto_backups_count(&self) -> usize {
        self.backups
            .iter()
            .filter(|b| !b.is_pinned && b.is_paste)
            .count()
    }

    /// Returns the count of unpinned backups for the character.
    #[inline]
    #[must_use]
    pub fn oldest_unpinned_auto_backup(&self) -> Option<&WoWCharacterBackup> {
        self.backups
            .iter()
            .filter(|b| !b.is_pinned && b.is_paste)
            .min_by_key(|b| b.timestamp)
    }
}

/// Represents the class of a World of Warcraft character.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum WoWClass {
    #[default]
    Unknown = 0,
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    DeathKnight = 6,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Monk = 10,
    Druid = 11,
    DemonHunter = 12,
    Evoker = 13,
}

impl WoWClass {
    /// Minimum valid Class ID.
    pub const MIN: u8 = Self::Warrior as u8;
    /// Maximum valid Class ID.
    pub const MAX: u8 = Self::Evoker as u8;

    /// Checks if the given class ID is valid.
    #[inline]
    #[must_use]
    pub fn is_valid_class_id(id: u8) -> bool {
        (Self::MIN..=Self::MAX).contains(&id)
    }

    /// Creates a `Class` from a given ID.
    #[inline]
    #[must_use]
    pub fn from_id(id: u8) -> Self {
        if Self::is_valid_class_id(id) {
            unsafe { std::mem::transmute::<u8, Self>(id) }
        } else if id == 0 {
            Self::Priest
        } else {
            Self::Unknown
        }
    }

    /// Returns the name of the class as a static string.
    #[inline]
    #[must_use]
    pub const fn class_name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Warrior => "Warrior",
            Self::Paladin => "Paladin",
            Self::Hunter => "Hunter",
            Self::Rogue => "Rogue",
            Self::Priest => "Priest",
            Self::DeathKnight => "Death Knight",
            Self::Shaman => "Shaman",
            Self::Mage => "Mage",
            Self::Warlock => "Warlock",
            Self::Monk => "Monk",
            Self::Druid => "Druid",
            Self::DemonHunter => "Demon Hunter",
            Self::Evoker => "Evoker",
        }
    }

    /// Returns the short name (abbreviation) of the class as a static
    /// string.
    #[inline]
    #[must_use]
    pub const fn class_short_name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unk",
            Self::Warrior => "War",
            Self::Paladin => "Pala",
            Self::Hunter => "Hunter",
            Self::Rogue => "Rogue",
            Self::Priest => "Priest",
            Self::DeathKnight => "DK",
            Self::Shaman => "Sha",
            Self::Mage => "Mage",
            Self::Warlock => "Lock",
            Self::Monk => "Monk",
            Self::Druid => "Druid",
            Self::DemonHunter => "DH",
            Self::Evoker => "Evoker",
        }
    }

    /// Returns the RGB colour associated with the class.
    #[inline]
    #[must_use]
    pub const fn class_colour(&self) -> Color {
        match self {
            Self::Unknown => crate::palette::UNKNOWN_COL,
            Self::Warrior => crate::palette::WARRIOR_COL,
            Self::Paladin => crate::palette::PALADIN_COL,
            Self::Hunter => crate::palette::HUNTER_COL,
            Self::Rogue => crate::palette::ROGUE_COL,
            Self::Priest => crate::palette::PRIEST_COL,
            Self::DeathKnight => crate::palette::DEATHKNIGHT_COL,
            Self::Shaman => crate::palette::SHAMAN_COL,
            Self::Mage => crate::palette::MAGE_COL,
            Self::Warlock => crate::palette::WARLOCK_COL,
            Self::Monk => crate::palette::MONK_COL,
            Self::Druid => crate::palette::DRUID_COL,
            Self::DemonHunter => crate::palette::DEMONHUNTER_COL,
            Self::Evoker => crate::palette::EVOKER_COL,
        }
    }
}
