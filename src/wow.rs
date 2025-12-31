use std::path::{Path, PathBuf};

use const_format::concatcp;
use itertools::Itertools;
use prost::Message;

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
const WOW_RETAIL_IDENT: &str = "retail";

/// Represents a World of Warcraft installation.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WowInstall {
    /// The Battle.net product code for this installation.
    pub product_code: String,
    /// The branch identifier for this installation (e.g., "retail", "classic", etc.).
    pub branch_ident: String,
    /// The root installation path for this World of Warcraft installation.
    pub install_path: String,
}

impl WowInstall {
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
}

/// Extract World of Warcraft installation data from a Battle.net product installation entry.
fn extract_wow_install_data(product: &productdb::ProductInstall) -> Option<WowInstall> {
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
    Some(WowInstall {
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
pub fn locate_wow_installs() -> Result<Vec<WowInstall>, Box<dyn std::error::Error>> {
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
const USER_DIR: &str = "WTF";
/// Name of the account directory within the `WoW` user settings.
const ACCOUNT_DIR: &str = "Account";
/// Name of the `SavedVariables` directory within the `WoW` user settings.
const SAVED_VARIABLES: &str = "SavedVariables";

/// Check if a directory name is a valid `WoW` account directory name within the `WTF/Account` path.
#[inline]
fn is_account_dir(dir_name: &str) -> bool {
    dir_name != SAVED_VARIABLES && dir_name.chars().all(|c| c.is_numeric() || c == '#')
}

/// Reads a directory and returns an iterator over all folders within it.
fn read_folders(
    dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = std::fs::DirEntry>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_dir(dir.as_ref())?
        .filter_map(Result::ok)
        .filter(|d| d.file_type().ok().is_some_and(|ft| ft.is_dir())))
}

/// Reads all folders in the given path and returns their names as a vector of strings.
fn read_folders_to_string(
    dir: impl AsRef<Path>,
) -> Result<impl Iterator<Item = String>, Box<dyn std::error::Error>> {
    Ok(read_folders(dir)?.filter_map(|d| Some(d.file_name().to_str()?.to_string())))
}

/// Finds all valid `WoW` account directories (not characters) in the given installation.
fn find_accounts_in_install(
    install: &WowInstall,
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

impl WowInstall {
    /// Returns the path to the user settings directory for this installation.
    #[inline]
    #[must_use]
    pub fn get_wtf_path(&self) -> PathBuf {
        let install_path = self.get_branch_path();
        install_path.join(USER_DIR)
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
                            .filter(|d| d != SAVED_VARIABLES)
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
    pub fn find_all_characters(&self) -> Option<Vec<WowCharacter>> {
        let realms = self.find_all_realms()?;
        let characters = realms
            .iter()
            .flat_map(|(account_name, realm_name)| {
                let realm_path = self.get_realm_path(account_name, realm_name);
                read_folders_to_string(realm_path).map_or_else(
                    |_| vec![],
                    |chars| {
                        chars
                            .map(|char_name| WowCharacter {
                                account: account_name.clone(),
                                branch: self.branch_ident.clone(),
                                name: char_name,
                                realm: realm_name.clone(),
                                class: Class::Unknown,
                                files: vec![],
                            })
                            .collect::<Vec<WowCharacter>>()
                    },
                )
            })
            .collect::<Vec<WowCharacter>>();
        Some(characters)
    }

    /// Find all realms characters across all realms across all accounts in this installation.
    /// This populates all file information from the character directories as well.
    #[inline]
    #[must_use]
    pub fn find_all_characters_and_files(&self) -> Option<Vec<WowCharacter>> {
        let mut chars = self.find_all_characters()?;
        for c in &mut chars {
            c.map_character_files(self);
        }
        Some(chars)
    }
}

/// Represents a file associated with a World of Warcraft character.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WowCharacterFile {
    pub name: String,
    pub stem: String,
    pub path: PathBuf,
    pub friendly_name: Option<String>,
}

impl WowCharacterFile {
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

    /// Returns true if the file has a friendly name associated with it.
    #[inline]
    #[must_use]
    pub const fn has_friendly_name(&self) -> bool {
        self.friendly_name.is_some()
    }
}

/// Represents a World of Warcraft character.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WowCharacter {
    pub account: String,
    pub branch: String,
    pub name: String,
    pub realm: String,
    pub class: Class,
    pub files: Vec<WowCharacterFile>,
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

/// Get a friendly name for a given filename, if available.
#[inline]
#[must_use]
fn get_friendly_name(filename: &str) -> Option<String> {
    FRIENDLY_NAMES
        .iter()
        .find(|(original_name, _)| filename == *original_name)
        .map(|(_, friendly_name)| friendly_name.to_string())
}

impl WowCharacter {
    /// Returns the path to the character's directory.
    #[inline]
    #[must_use]
    pub fn get_character_path(&self, install: &WowInstall) -> PathBuf {
        install
            .get_realm_path(&self.account, &self.realm)
            .join(&self.name)
    }

    /// Maps all files in the character's directory to the `files` field.
    /// Also populates the character class information if possible.
    #[inline]
    pub fn map_character_files(&mut self, install: &WowInstall) {
        let char_path = self.get_character_path(install);

        if !char_path.is_dir() || !char_path.exists() {
            return;
        }
        let Ok(files) = std::fs::read_dir(&char_path) else {
            return;
        };

        self.files = files
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
                Some(WowCharacterFile {
                    name,
                    stem,
                    path,
                    friendly_name: get_friendly_name(&entry.file_name().to_string_lossy()),
                })
            })
            .sorted_by(|af, bf| bf.has_friendly_name().cmp(&af.has_friendly_name()))
            .collect();

        self.files
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
                    self.class = Class::from_id(id);
                }
                Some(())
            });
    }
}

/// Represents the class of a World of Warcraft character.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Class {
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

impl Class {
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
    pub const fn class_colour(&self) -> (u8, u8, u8) {
        match self {
            Self::Unknown => (130, 130, 130),
            Self::Warrior => (198, 155, 109),
            Self::Paladin => (244, 140, 186),
            Self::Hunter => (170, 211, 144),
            Self::Rogue => (255, 244, 104),
            Self::Priest => (255, 255, 255),
            Self::DeathKnight => (196, 30, 58),
            Self::Shaman => (0, 112, 221),
            Self::Mage => (63, 199, 235),
            Self::Warlock => (135, 136, 238),
            Self::Monk => (0, 255, 152),
            Self::Druid => (255, 124, 10),
            Self::DemonHunter => (163, 48, 201),
            Self::Evoker => (51, 147, 127),
        }
    }
}
