use chrono::Local;
use zip::{
    ZipWriter,
    write::{FileOptions, FullFileOptions},
};

use crate::wow::{WowCharacter, WowInstall};
use std::fs as filesystem;
use std::path::{Path, PathBuf};

pub type BackendResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Ensure that a directory exists at the given path, creating it if necessary.
/// # Errors
/// Returns an error if the directory cannot be created if it does not exist.
pub fn ensure_directory(path: &Path) -> BackendResult<()> {
    if !path.exists() {
        filesystem::create_dir_all(path)?;
    }
    Ok(())
}

/// Type alias for a boxed iterator over directory entries.
pub type DirIterator = Box<dyn Iterator<Item = BackendResult<filesystem::DirEntry>>>;

/// Returns `Vec` containing all file paths recursively descending over all
/// files and folders in `base_path`.
/// # Errors
/// Returns an error if any I/O operation fails.
pub fn walk_dir_recursive(
    base_path: &Path,
    excluded_dirs: &[impl AsRef<Path>],
) -> BackendResult<Vec<PathBuf>> {
    fn walk_dir_impl(
        path: &Path,
        excluded_dirs: &[PathBuf],
        entries: &mut Vec<PathBuf>,
    ) -> BackendResult<()> {
        for entry in filesystem::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if excluded_dirs.contains(&path) {
                continue;
            }

            if path.is_file() {
                entries.push(path);
            } else if path.is_dir() {
                walk_dir_impl(&path, excluded_dirs, entries)?;
            }
        }
        Ok(())
    }

    let excluded_paths = excluded_dirs
        .iter()
        .map(|p| base_path.join(p.as_ref()))
        .collect::<Vec<_>>();

    let mut entries = Vec::new();
    walk_dir_impl(base_path, &excluded_paths, &mut entries)?;
    Ok(entries)
}

/// Generate a backup file name for the given `WoW` character.
#[inline]
#[must_use]
pub fn get_backup_name(character: &WowCharacter) -> String {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    format!("{}_{}.zip", character.name, timestamp)
}

/// Create a backup ZIP archive of the given `WoW` character's data.
/// # Errors
/// Returns an error if any file operations fail.
pub fn backup_character(
    character: &WowCharacter,
    install: &WowInstall,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let char_path = character.get_character_path(install);
    let backup_dir = character.get_backups_dir(install);
    ensure_directory(&backup_dir)?;

    let backup_file_name = get_backup_name(character);
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
    }

    zip.finish()?;
    Ok(backup_file_path)
}

/// Create a backup ZIP archive of the given `WoW` character's, backing up only the selected
/// files.
/// # Errors
/// Returns an error if any file operations fail.
pub fn backup_character_files(
    character: &WowCharacter,
    selected_files: &[PathBuf],
    install: &WowInstall,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let char_path = character.get_character_path(install);
    let backup_dir = char_path.join(crate::wow::BACKUPS_DIR_NAME);
    ensure_directory(&backup_dir)?;

    let backup_file_name = get_backup_name(character);
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
    }

    zip.finish()?;
    Ok(backup_file_path)
}
