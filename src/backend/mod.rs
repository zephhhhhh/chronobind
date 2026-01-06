pub mod task;

use std::sync::mpsc::Sender as MPSCSender;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use itertools::Itertools;
use zip::{
    ZipWriter,
    read::ZipArchive,
    write::{FileOptions, FullFileOptions},
};

use crate::{
    backend::task::{IOProgress, IOTask},
    files::AnyResult,
    tui_log::mock_prefix,
    wow::{WoWCharacter, WoWCharacterBackup, WoWInstall},
};

use std::fs as filesystem;
use std::path::PathBuf;

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
pub fn get_backup_name(character: &WoWCharacter, paste: bool, pinned: bool) -> String {
    get_backup_name_from(&character.name, Local::now(), paste, pinned)
}

/// A structure representing a `WoW` character along with its associated install,
/// able to be owned locally and moved across threads.
#[derive(Debug, Clone)]
pub struct CharWithInstallLocal {
    pub character: WoWCharacter,
    pub install: WoWInstall,
}

impl CharWithInstallLocal {
    /// Get the character's data path.
    #[must_use]
    pub fn get_character_path(&self) -> PathBuf {
        self.character.get_character_path(&self.install)
    }

    /// Get the character's backups directory path.
    #[must_use]
    pub fn get_backups_dir(&self) -> PathBuf {
        self.character.get_backups_dir(&self.install)
    }
}

impl From<crate::ui::CharacterWithInstall<'_>> for CharWithInstallLocal {
    fn from(ci: crate::ui::CharacterWithInstall<'_>) -> Self {
        Self {
            character: ci.0.character.clone(),
            install: ci.1.clone(),
        }
    }
}

fn backup_character_async_internal(
    tx: &MPSCSender<IOProgress>,
    src_char: &CharWithInstallLocal,
    selected_files: Option<&[PathBuf]>,
    paste: bool,
    pinned: bool,
    mock_mode: bool,
) -> AnyResult<()> {
    let char_path = src_char.get_character_path();
    let backup_dir = src_char.get_backups_dir();

    ensure_directory(&backup_dir, mock_mode)?;

    let mut dir_iter = walk_dir_recursive(&char_path, &[crate::wow::BACKUPS_DIR_NAME])?;
    if let Some(selected) = selected_files {
        let fully_qualified_paths: Vec<PathBuf> =
            selected.iter().map(|p| char_path.join(p)).collect();
        dir_iter.retain(|p| fully_qualified_paths.contains(p));
    }

    let total = dir_iter.len();

    let backup_file_name = get_backup_name(&src_char.character, paste, pinned);
    let backup_file_path = backup_dir.join(backup_file_name);
    let file = filesystem::File::create(&backup_file_path)?;
    let options: FullFileOptions =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(file);

    for (files_backed_up, file_path) in dir_iter.iter().enumerate() {
        let relative_path = file_path.strip_prefix(&char_path)?;
        zip.start_file(relative_path.to_string_lossy(), options.clone())?;

        if !mock_mode {
            let mut f = filesystem::File::open(file_path)?;
            std::io::copy(&mut f, &mut zip)?;
        }

        log::info!("Backed up `{}`", relative_path.display());
        tx.send(IOProgress::Advanced {
            completed: files_backed_up.saturating_add(1),
            total,
        })?;
    }

    zip.finish()?;

    log::debug!("Finished backup to `{}`", backup_file_path.display());
    tx.send(IOProgress::Finished)?;

    Ok(())
}

/// Create a backup ZIP archive of the given `WoW` character's data.
/// # Errors
/// Returns an error if any file operations fail.
#[must_use]
pub fn backup_character_all_async(
    src_char: CharWithInstallLocal,
    paste: bool,
    pinned: bool,
    mock_mode: bool,
) -> IOTask {
    IOTask::new(move |tx| {
        backup_character_async_internal(tx, &src_char, None, paste, pinned, mock_mode)
    })
    .name("Backing up all files")
}

/// Create a backup ZIP archive of the given `WoW` character's data, backing up only the selected
/// files.
/// # Errors
/// Returns an error if any file operations fail.
#[must_use]
pub fn backup_character_selected_async(
    src_char: CharWithInstallLocal,
    selected_files: &[PathBuf],
    paste: bool,
    pinned: bool,
    mock_mode: bool,
) -> IOTask {
    let sel_files = selected_files.to_vec();

    IOTask::new(move |tx| {
        backup_character_async_internal(tx, &src_char, Some(&sel_files), paste, pinned, mock_mode)
    })
    .name("Backing up selected files")
}

/// Create a backup ZIP archive of the given `WoW` character's data, optionally with selected files.
/// # Errors
/// Returns an error if any file operations fail.
#[must_use]
pub fn backup_character_async(
    src_char: crate::ui::CharacterWithInstall<'_>,
    selected_files: Option<&[PathBuf]>,
    paste: bool,
    pinned: bool,
    mock_mode: bool,
) -> IOTask {
    selected_files.map_or_else(
        || backup_character_all_async(src_char.into(), paste, pinned, mock_mode),
        |selected| {
            backup_character_selected_async(src_char.into(), selected, paste, pinned, mock_mode)
        },
    )
}

fn paste_character_files_async_internal(
    dest_character: CharWithInstallLocal,
    src_character: CharWithInstallLocal,
    selected_files: &[PathBuf],
    mock_mode: bool,
) -> IOTask {
    let sel_files = selected_files.to_vec();

    IOTask::new(move |tx| {
        let dest_char_path = dest_character.get_character_path();
        let src_char_path = src_character.get_character_path();

        let total = sel_files.len();

        for (files_copied, relative_path) in sel_files.iter().enumerate() {
            let src_file_path = src_char_path.join(relative_path);
            let dest_file_path = dest_char_path.join(relative_path);

            if !mock_mode {
                filesystem::copy(&src_file_path, &dest_file_path)?;
            }

            log::info!(
                "{}Copied `{}` to `{}`",
                mock_prefix(mock_mode),
                relative_path.display(),
                dest_file_path.display()
            );
            tx.send(IOProgress::Advanced {
                completed: files_copied.saturating_add(1),
                total,
            })?;
        }

        tx.send(IOProgress::Finished)?;

        Ok(())
    })
    .name("Pasting character files")
}

/// Paste the selected files from the source `WoW` character to the destination `WoW` character.
/// Will create a backup of the replaced files in the destination character before pasting.
/// # Errors
/// Returns an error if any file operations fail.
/// # Parameters
/// - `dest_character`: The destination character to which files will be pasted.
/// - `src_character`: The source character from which files will be copied.
/// - `selected_files`: A list of relative file paths to be copied.
/// - `mock_mode`: If true, no actual file operations will be performed; only logging will occur.
#[must_use]
pub fn paste_character_files_async(
    dest_character: CharWithInstallLocal,
    src_character: CharWithInstallLocal,
    selected_files: &[PathBuf],
    mock_mode: bool,
) -> IOTask {
    let first_task = if mock_mode {
        None
    } else {
        log::debug!("Backing up files before paste...");
        Some(backup_character_selected_async(
            dest_character.clone(),
            selected_files,
            true,
            false,
            mock_mode,
        ))
    };

    let paste_task = paste_character_files_async_internal(
        dest_character,
        src_character,
        selected_files,
        mock_mode,
    );

    if let Some(backup_task) = first_task {
        backup_task.then(paste_task)
    } else {
        paste_task
    }
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
#[must_use]
pub fn restore_backup_async(
    character: CharWithInstallLocal,
    backup_path: PathBuf,
    mock_mode: bool,
) -> IOTask {
    IOTask::new(move |tx| {
        let file = filesystem::File::open(backup_path)?;
        let mut archive = ZipArchive::new(file)?;

        let backup_files_count = archive.file_names().count();

        let dest_root = character.get_character_path();
        ensure_directory(&dest_root, mock_mode)?;

        let mut files_restored = 0;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;

            let Some(rel_path) = entry.enclosed_name() else {
                log::warn!(
                    "{}Skipped extracting file with invalid path: `{}`",
                    mock_prefix(mock_mode),
                    entry.name()
                );
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

            tx.send(IOProgress::Advanced {
                completed: files_restored,
                total: backup_files_count,
            })?;

            log::info!(
                "{}Restored file `{}`",
                mock_prefix(mock_mode),
                rel_path.display()
            );
        }

        tx.send(IOProgress::Finished)?;

        Ok(())
    })
    .name("Restoring backup")
}

/// Change the pinned status of a backup for the given `WoW` character at the specified index.
/// # Errors
/// Returns an error if any file operations fail.
pub fn change_backup_pin_state(
    backup: &WoWCharacterBackup,
    pinned: bool,
    mock_mode: bool,
) -> AnyResult<()> {
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
pub fn toggle_backup_pin(backup: &WoWCharacterBackup, mock_mode: bool) -> AnyResult<()> {
    let new_pinned = !backup.is_pinned;
    change_backup_pin_state(backup, new_pinned, mock_mode)
}

/// Manage automatic backups for the given `WoW` character, removing oldest unpinned backups
/// if the maximum allowed number is exceeded.
/// # Errors
/// Returns an error if any file operations fail.
#[must_use]
pub fn manage_character_backups(
    character: crate::ui::CharacterWithInstall<'_>,
    max_auto_backups: usize,
    mock_mode: bool,
) -> Option<IOTask> {
    let auto_backups: Vec<WoWCharacterBackup> = character.0.character.unpinned_auto_backups();
    let auto_backups_count = auto_backups.len();
    let backups_to_clean_count = auto_backups_count.saturating_sub(max_auto_backups);

    log::debug!(
        "Character `{}` has {} unpinned automatic backups, total backups: {}.",
        character.0.character.name,
        auto_backups.len(),
        character.0.character.backups.len()
    );

    if backups_to_clean_count > 0 {
        log::info!(
            "Character `{}` has {} unpinned automatic backups, exceeding the maximum of {}. Removing oldest backups...",
            character.0.character.name,
            character.0.character.unpinned_auto_backups_count(),
            max_auto_backups
        );
    } else {
        return None;
    }

    Some(
        IOTask::new(move |tx| {
            let backups_to_clean = auto_backups
                .iter()
                .sorted_by(|a, b| a.timestamp.cmp(&b.timestamp))
                .take(backups_to_clean_count)
                .collect::<Vec<_>>();

            let mut removed_count = 0;
            for backup in &backups_to_clean {
                if delete_backup_file(backup, true, mock_mode)? {
                    removed_count += 1;
                }

                tx.send(IOProgress::Advanced {
                    completed: removed_count,
                    total: backups_to_clean_count,
                })?;
            }

            tx.send(IOProgress::Finished)?;

            Ok(())
        })
        .name("Cleaning automatic backups"),
    )
}

/// Manage automatic backups for the given `WoW` character, removing oldest unpinned backups
/// if the maximum allowed number is exceeded.
/// # Errors
/// Returns an error if any file operations fail.
pub fn delete_backup_file(
    backup: &WoWCharacterBackup,
    auto_removed: bool,
    mock_mode: bool,
) -> AnyResult<bool> {
    let bad_removal = auto_removed && backup.is_pinned;
    if !mock_mode && !bad_removal {
        std::fs::remove_file(&backup.path)?;
    }
    if bad_removal {
        log::warn!(
            "{}Attempted to auto-remove pinned backup `{}`; operation skipped.",
            mock_prefix(mock_mode),
            backup.formatted_name()
        );
    } else {
        log::info!(
            "{}Deleted{} backup `{}`",
            mock_prefix(mock_mode),
            if auto_removed { " old" } else { "" },
            backup.formatted_name()
        );
    }
    Ok(!bad_removal)
}
