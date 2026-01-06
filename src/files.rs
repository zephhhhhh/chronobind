use filesystem::DirEntry;
use std::error::Error;
use std::fs as filesystem;
use std::path::{Path, PathBuf};

use crate::tui_log::mock_prefix;

pub type AnyResult<T> = Result<T, Box<dyn Error>>;

/// Reads a directory and returns an iterator over all folders within it.
/// # Errors
/// Returns an error if there are any I/O issues.
pub fn read_folders(dir: impl AsRef<Path>) -> AnyResult<impl Iterator<Item = DirEntry>> {
    Ok(std::fs::read_dir(dir.as_ref())?
        .filter_map(Result::ok)
        .filter(|d| d.file_type().ok().is_some_and(|ft| ft.is_dir())))
}

/// Reads all folders in the given path and returns their names as a vector of strings.
/// # Errors
/// Returns an error if there are any I/O issues.
pub fn read_folders_to_string(dir: impl AsRef<Path>) -> AnyResult<impl Iterator<Item = String>> {
    Ok(read_folders(dir)?.filter_map(|d| Some(d.file_name().to_str()?.to_string())))
}

/// Reads a directory and returns an iterator over all files within it.
/// # Errors
/// Returns an error if there are any I/O issues.
pub fn read_files(dir: impl AsRef<Path>) -> AnyResult<impl Iterator<Item = DirEntry>> {
    Ok(std::fs::read_dir(dir.as_ref())?
        .filter_map(Result::ok)
        .filter(|d| d.file_type().ok().is_some_and(|ft| ft.is_file())))
}

/// Reads a directory and returns an iterator over all files within it.
/// # Errors
/// Returns an error if there are any I/O issues.
pub fn read_files_to_string(dir: impl AsRef<Path>) -> AnyResult<impl Iterator<Item = String>> {
    Ok(read_files(dir)?.filter_map(|d| Some(d.file_name().to_str()?.to_string())))
}

/// Ensure that a directory exists at the given path, creating it if necessary.
/// # Errors
/// Returns an error if the directory cannot be created if it does not exist.
pub fn ensure_directory(path: &Path, mock_mode: bool) -> AnyResult<()> {
    if !path.exists() {
        if !mock_mode {
            filesystem::create_dir_all(path)?;
        }
        log::info!(
            "{}Created directory: {}",
            mock_prefix(mock_mode),
            path.display()
        );
    }
    Ok(())
}

/// Returns `Vec` containing all file paths recursively descending over all
/// files and folders in `base_path`.
/// # Errors
/// Returns an error if any I/O operation fails.
pub fn walk_dir_recursive<T: AsRef<Path>>(
    base_path: &Path,
    excluded_dirs: &[T],
) -> AnyResult<Vec<PathBuf>> {
    fn walk_dir_impl(
        path: &Path,
        excluded_dirs: &[PathBuf],
        entries: &mut Vec<PathBuf>,
    ) -> AnyResult<()> {
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
