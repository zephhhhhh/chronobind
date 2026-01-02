use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use zip::{
    ZipWriter,
    read::ZipArchive,
    write::{FileOptions, FullFileOptions},
};

use crate::{
    files::AnyResult,
    tui_log::mock_prefix,
    wow::{WowBackup, WowCharacter, WowInstall},
};

use std::path::PathBuf;
use std::{fs as filesystem, path::Path};

use crate::files::{ensure_directory, walk_dir_recursive};

/// Suffix to append to backup files created during a paste operation.
const PASTE_IDENT: &str = "RESTORE";
/// Suffix to append to backup files that are pinned to not be auto-removed.
const PINNED_IDENT: &str = "PINNED";

/// Time format used in backup file names.
pub const BACKUP_FILE_TIME_FORMAT: &str = "%Y%m%d-%H%M%S";
/// Display time format used in backup listings.
pub const DISPLAY_TIME_FORMAT: &str = "%d/%m/%y %H:%M";

/// File extension for backup files.
pub const BACKUP_FILE_EXTENSION: &str = "zip";

/// Convert an `OsStr` to a `String`, handling possible invalid UTF-8.
#[inline]
#[must_use]
pub fn os_str_to_string(s: &std::ffi::OsStr) -> String {
    s.to_string_lossy().into_owned()
}

/// Generate a backup file name for the given parameters.
#[inline]
#[must_use]
pub fn get_backup_name_from(
    char_name: &str,
    timestamp: DateTime<Local>,
    paste: bool,
    pinned: bool,
) -> String {
    let ts_str = timestamp.format(BACKUP_FILE_TIME_FORMAT);
    format!(
        "{}_{}{}{}.{BACKUP_FILE_EXTENSION}",
        char_name,
        ts_str,
        if paste {
            format!("_{PASTE_IDENT}")
        } else {
            String::new()
        },
        if pinned {
            format!("_{PINNED_IDENT}")
        } else {
            String::new()
        }
    )
}

/// Generate a backup file name for the given `WoW` character.
#[inline]
#[must_use]
pub fn get_backup_name(character: &WowCharacter, paste: bool, pinned: bool) -> String {
    get_backup_name_from(&character.name, Local::now(), paste, pinned)
}

/// A structure representing a `WoW` character along with its associated install.
#[derive(Debug, Clone)]
pub struct CharacterWithInstall<'a> {
    pub character: &'a WowCharacter,
    pub install: &'a WowInstall,
}

impl CharacterWithInstall<'_> {
    /// Get the character's data path.
    #[must_use]
    pub fn get_character_path(&self) -> PathBuf {
        self.character.get_character_path(self.install)
    }

    /// Get the character's backups directory path.
    #[must_use]
    pub fn get_backups_dir(&self) -> PathBuf {
        self.character.get_backups_dir(self.install)
    }
}

/// Create a backup ZIP archive of the given `WoW` character's data.
/// # Errors
/// Returns an error if any file operations fail.
pub fn backup_character(
    character: &CharacterWithInstall,
    paste: bool,
    pinned: bool,
) -> AnyResult<PathBuf> {
    let char_path = character.get_character_path();
    let backup_dir = character.get_backups_dir();
    ensure_directory(&backup_dir, false)?;

    let backup_file_name = get_backup_name(character.character, paste, pinned);
    let backup_file_path = backup_dir.join(backup_file_name);
    let file = filesystem::File::create(&backup_file_path)?;
    let options: FullFileOptions =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(file);

    for file_path in walk_dir_recursive(&char_path, &[crate::wow::BACKUPS_DIR_NAME])? {
        let relative_path = file_path.strip_prefix(&char_path)?;
        zip.start_file(relative_path.to_string_lossy(), options.clone())?;
        let mut f = filesystem::File::open(&file_path)?;
        std::io::copy(&mut f, &mut zip)?;

        log::info!("Backed up `{}`", relative_path.display());
    }

    zip.finish()?;

    log::debug!("Finished backup to `{}`", backup_file_path.display());

    Ok(backup_file_path)
}

/// Create a backup ZIP archive of the given `WoW` character's, backing up only the selected
/// files.
/// # Errors
/// Returns an error if any file operations fail.
pub fn backup_character_files(
    character: &CharacterWithInstall,
    selected_files: &[PathBuf],
    paste: bool,
    pinned: bool,
) -> AnyResult<PathBuf> {
    let char_path = character.get_character_path();
    let backup_dir = char_path.join(crate::wow::BACKUPS_DIR_NAME);
    ensure_directory(&backup_dir, false)?;

    let backup_file_name = get_backup_name(character.character, paste, pinned);
    let backup_file_path = backup_dir.join(backup_file_name);
    let file = filesystem::File::create(&backup_file_path)?;
    let options: FullFileOptions =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(file);

    let fully_qualified_paths: Vec<PathBuf> =
        selected_files.iter().map(|p| char_path.join(p)).collect();

    for file_path in walk_dir_recursive(&char_path, &[crate::wow::BACKUPS_DIR_NAME])? {
        if !fully_qualified_paths.contains(&file_path) {
            continue;
        }
        let relative_path = file_path.strip_prefix(&char_path)?;
        zip.start_file(relative_path.to_string_lossy(), options.clone())?;
        let mut f = filesystem::File::open(&file_path)?;
        std::io::copy(&mut f, &mut zip)?;

        log::info!("Backed up `{}`", relative_path.display());
    }

    zip.finish()?;

    log::info!(
        "Finished selective backup to `{}`",
        backup_file_path.display()
    );

    Ok(backup_file_path)
}

/// Create a backup ZIP archive of the given `WoW` character's, backing up only the selected
/// files.
/// # Errors
/// Returns an error if any file operations fail.
/// # Parameters
/// - `dest_character`: The destination character to which files will be pasted.
/// - `src_character`: The source character from which files will be copied.
/// - `selected_files`: A list of relative file paths to be copied.
/// - `mock_mode`: If true, no actual file operations will be performed; only logging will occur.
pub fn paste_character_files(
    dest_character: &CharacterWithInstall,
    src_character: &CharacterWithInstall,
    selected_files: &[PathBuf],
    mock_mode: bool,
) -> AnyResult<usize> {
    if !mock_mode {
        log::debug!("Backing up files before paste...");
        backup_character_files(dest_character, selected_files, true, false)?;
        log::debug!("Done.");
    }

    let dest_char_path = dest_character.get_character_path();
    let src_char_path = src_character.get_character_path();

    let mut files_copied = 0;

    for relative_path in selected_files {
        let src_file_path = src_char_path.join(relative_path);
        let dest_file_path = dest_char_path.join(relative_path);

        if !mock_mode {
            filesystem::copy(&src_file_path, &dest_file_path)?;
            files_copied += 1;
        }

        log::info!(
            "{}Copied `{}` to `{}`",
            mock_prefix(mock_mode),
            relative_path.display(),
            dest_file_path.display()
        );
    }

    log::debug!("{}Pasted {files_copied} files.", mock_prefix(mock_mode));

    Ok(files_copied)
}

/// Extract the character name and timestamp from a backup file path.
#[must_use]
pub fn extract_backup_name(backup_filestem: &str) -> Option<(String, DateTime<Local>, bool, bool)> {
    let segments = backup_filestem.split('_').collect::<Vec<&str>>();
    if segments.len() < 2 {
        return None;
    }
    let name = segments[0].to_string();
    let date = NaiveDateTime::parse_from_str(segments[1], BACKUP_FILE_TIME_FORMAT).ok()?;
    let remaining_segments = segments.len().saturating_sub(2);

    let mut paste = false;
    let mut pinned = false;

    for i in 0..remaining_segments {
        match segments[2 + i] {
            PASTE_IDENT => paste = true,
            PINNED_IDENT => pinned = true,
            _ => {}
        }
    }

    Some((
        name,
        Local.from_local_datetime(&date).unwrap(),
        paste,
        pinned,
    ))
}

/// Restore a backup for the given `WoW` character from the specified backup file path.
/// # Errors
/// Returns an error if any file operations fail.
pub fn restore_backup(
    character: &CharacterWithInstall,
    backup_path: &Path,
    mock_mode: bool,
) -> AnyResult<usize> {
    let file = filesystem::File::open(backup_path)?;
    let mut archive = ZipArchive::new(file)?;

    let dest_root = character.get_character_path();
    ensure_directory(&dest_root, mock_mode)?;

    let mut files_restored = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;

        let Some(rel_path) = entry.enclosed_name() else {
            continue;
        };

        let out_path = dest_root.join(&rel_path);

        if entry.name().ends_with('/') {
            ensure_directory(&out_path, mock_mode)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            ensure_directory(parent, mock_mode)?;
        }

        if !mock_mode {
            let mut outfile = filesystem::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
            files_restored += 1;
        }

        log::info!(
            "{}Restored file `{}`",
            mock_prefix(mock_mode),
            rel_path.display()
        );
    }

    Ok(files_restored)
}

/// Change the pinned status of a backup for the given `WoW` character at the specified index.
/// # Errors
/// Returns an error if any file operations fail.
pub fn change_backup_pin_state(backup: &WowBackup, pinned: bool, mock_mode: bool) -> AnyResult<()> {
    if backup.is_pinned == pinned {
        log::debug!(
            "Pinned state is already `{pinned}` on backup `{}` no change needed.",
            backup.formatted_name()
        );
        return Ok(());
    }

    let og_path = backup
        .path
        .file_name()
        .map(os_str_to_string)
        .unwrap_or_default();
    let new_backup_name =
        get_backup_name_from(&backup.char_name, backup.timestamp, backup.is_paste, pinned);

    if !mock_mode {
        std::fs::rename(
            backup.path.as_path(),
            backup.path.with_file_name(&new_backup_name),
        )?;
    }

    log::info!(
        "{}Renamed backup `{}` from `{}` to `{}`",
        mock_prefix(mock_mode),
        backup.formatted_name(),
        og_path,
        new_backup_name
    );

    Ok(())
}

/// Toggle the pinned status of a backup for the given `WoW` character at the specified index.
/// # Errors
/// Returns an error if any file operations fail.
pub fn toggle_backup_pin(backup: &WowBackup, mock_mode: bool) -> AnyResult<()> {
    let new_pinned = !backup.is_pinned;
    change_backup_pin_state(backup, new_pinned, mock_mode)
}

/// Manage automatic backups for the given `WoW` character, removing oldest unpinned backups
/// if the maximum allowed number is exceeded.
/// # Errors
/// Returns an error if any file operations fail.
pub fn manage_character_backups(
    character: &CharacterWithInstall,
    max_auto_backups: usize,
    mock_mode: bool,
) -> AnyResult<usize> {
    let mut auto_backups: Vec<WowBackup> = character.character.unpinned_auto_backups();

    log::debug!(
        "Character `{}` has {} unpinned automatic backups, total backups: {}.",
        character.character.name,
        auto_backups.len(),
        character.character.backups.len()
    );

    if auto_backups.len() >= max_auto_backups {
        log::info!(
            "Character `{}` has {} unpinned automatic backups, exceeding the maximum of {}. Removing oldest backups...",
            character.character.name,
            character.character.unpinned_auto_backups_count(),
            max_auto_backups
        );
    } else {
        return Ok(0);
    }

    let mut removed_count = 0;

    while auto_backups.len() > max_auto_backups {
        if let Some((oldest_index, oldest_backup)) = auto_backups
            .iter()
            .enumerate()
            .min_by_key(|(_, b)| b.timestamp)
        {
            if !mock_mode && !oldest_backup.is_pinned {
                std::fs::remove_file(&oldest_backup.path)?;
                removed_count += 1;
            }
            log::info!(
                "{}Removed old backup `{}`",
                mock_prefix(mock_mode),
                oldest_backup.formatted_name()
            );
            auto_backups.remove(oldest_index);
        }
    }

    Ok(removed_count)
}
