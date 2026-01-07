use filesystem::File;
use std::path::Path;
use std::sync::Mutex;
use std::{fs as filesystem, sync::Arc};
use zip::ZipArchive;
use zip::read::ZipFile;

use zip::{ZipWriter, write::FileOptions};

use crate::files::AnyResult;

/// A simple ZIP writer wrapper/interface for creating backups.
#[derive(Debug)]
#[must_use]
pub struct ChronoZipWriter<'a> {
    zip: Option<ZipWriter<File>>,
    options: FileOptions<'a, ()>,
}

impl ChronoZipWriter<'_> {
    pub const DEFAULT_ZIP_OPTIONS: FileOptions<'static, ()> =
        FileOptions::DEFAULT.compression_method(zip::CompressionMethod::Deflated);
}

impl<'a> ChronoZipWriter<'a> {
    /// Create a new `ChronoZipWriter` with the specified file and options.
    /// # Errors
    /// Returns an error if the file cannot be created.
    pub fn new(path: &Path, mock_mode: bool) -> AnyResult<Self> {
        let zip = if mock_mode {
            log::debug!(
                "Mock mode enabled, skipping creation of zip file at `{}`",
                path.display()
            );
            None
        } else {
            let file = filesystem::File::create(path)?;
            Some(ZipWriter::new(file))
        };
        Ok(Self {
            zip,
            options: Self::DEFAULT_ZIP_OPTIONS,
        })
    }

    /// Create a new `ChronoZipWriter` wrapped in an `Arc<Mutex<>>` with the specified file and options.
    /// # Errors
    /// Returns an error if the file cannot be created.
    #[must_use]
    pub fn new_arc(path: &Path, mock_mode: bool) -> Option<Arc<Mutex<Self>>> {
        match Self::new(path, mock_mode) {
            Ok(writer) => Some(Arc::new(Mutex::new(writer))),
            Err(e) => {
                log::error!(
                    "Failed to create ChronoZipWriter for path `{}`: {}",
                    path.display(),
                    e
                );
                None
            }
        }
    }

    /// Set the file options for the ZIP writer.
    pub const fn with_options(mut self, options: FileOptions<'a, ()>) -> Self {
        self.options = options;
        self
    }
}

impl ChronoZipWriter<'_> {
    /// Check if the writer is in mock mode.
    #[must_use]
    pub const fn is_mock_mode(&self) -> bool {
        self.zip.is_none()
    }
}

impl ChronoZipWriter<'_> {
    /// Create a new directory in the zip file, does nothing if in mock mode.
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn add_directory<S: Into<String>>(&mut self, name: S) -> AnyResult<()> {
        if let Some(zip) = self.zip.as_mut() {
            zip.add_directory(name.into(), self.options)?;
        }
        Ok(())
    }

    /// Create a new directory in the zip file, does nothing if in mock mode.
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn add_directory_from_path<P: AsRef<Path>>(&mut self, name: P) -> AnyResult<()> {
        if let Some(zip) = self.zip.as_mut() {
            zip.add_directory_from_path(name.as_ref(), self.options)?;
        }
        Ok(())
    }

    /// Start a new file in the ZIP archive, does nothing if in mock mode.
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn start_file<S: Into<String>>(&mut self, name: S) -> AnyResult<()> {
        if let Some(zip) = self.zip.as_mut() {
            zip.start_file(name.into(), self.options)?;
        }
        Ok(())
    }

    /// Start a new file in the ZIP archive, does nothing if in mock mode.
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn copy_file<S: Into<String>, P: AsRef<Path>>(
        &mut self,
        name: S,
        source_path: P,
    ) -> AnyResult<()> {
        self.start_file(name)?;
        if let Some(zip) = self.zip.as_mut() {
            let mut f = filesystem::File::open(source_path.as_ref())?;
            std::io::copy(&mut f, zip)?;
        }
        Ok(())
    }

    /// Finish writing the ZIP archive, does nothing if in mock mode.
    /// # Errors
    /// Returns an error if the operation fails.
    pub fn finish(&mut self) -> AnyResult<()> {
        if let Some(zip) = self.zip.take() {
            zip.finish()?;
        }
        Ok(())
    }
}

impl Drop for ChronoZipWriter<'_> {
    fn drop(&mut self) {
        if let Some(zip) = self.zip.take() {
            match zip.finish() {
                Ok(_) => {
                    log::debug!("Successfully finished ZIP archive on drop.");
                }
                Err(e) => {
                    log::error!("Failed to finish ZIP archive: {e}");
                }
            }
        }
    }
}

/// A simple ZIP writer wrapper/interface for creating backups.
#[derive(Debug)]
#[must_use]
pub struct ChronoZipReader<'a> {
    archive: ZipArchive<File>,
    options: FileOptions<'a, ()>,
}

impl ChronoZipReader<'_> {
    pub const DEFAULT_ZIP_OPTIONS: FileOptions<'static, ()> =
        FileOptions::DEFAULT.compression_method(zip::CompressionMethod::Deflated);
}

impl<'a> ChronoZipReader<'a> {
    /// Create a new `ChronoZipReader` with the specified file and options.
    /// # Errors
    /// Returns an error if the file cannot be created.
    pub fn new(path: &Path) -> AnyResult<Self> {
        let file = filesystem::File::open(path)?;
        let archive = ZipArchive::new(file)?;
        Ok(Self {
            archive,
            options: Self::DEFAULT_ZIP_OPTIONS,
        })
    }

    /// Create a new `ChronoZipReader` wrapped in an `Arc<Mutex<>>` with the specified file and options.
    /// # Errors
    /// Returns an error if the file cannot be created.
    #[must_use]
    pub fn new_arc(path: &Path) -> Option<Arc<Mutex<Self>>> {
        match Self::new(path) {
            Ok(writer) => Some(Arc::new(Mutex::new(writer))),
            Err(e) => {
                log::error!(
                    "Failed to create ChronoZipReader for path `{}`: {}",
                    path.display(),
                    e
                );
                None
            }
        }
    }

    /// Set the file options for the ZIP reader.
    pub const fn with_options(mut self, options: FileOptions<'a, ()>) -> Self {
        self.options = options;
        self
    }
}

impl ChronoZipReader<'_> {
    /// Get a file by its index in the ZIP archive.
    /// # Errors
    /// Returns an error if the ZIP read or access fails.
    #[inline]
    pub fn by_index(&mut self, index: usize) -> AnyResult<ZipFile<'_, filesystem::File>> {
        Ok(self.archive.by_index(index)?)
    }

    /// Search for a file entry by name
    /// # Errors
    /// Returns an error if the ZIP read or access fails.
    pub fn by_name(&mut self, name: &str) -> AnyResult<ZipFile<'_, filesystem::File>> {
        Ok(self.archive.by_name(name)?)
    }

    /// Get an iterator over the file names and directories in the ZIP archive.
    #[inline]
    pub fn file_names(&mut self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    /// Get an iterator over the directories in the root directory of the ZIP archive.
    #[inline]
    pub fn directories_in_root(&mut self) -> Vec<String> {
        let mut dirs = std::collections::HashSet::new();

        for name in self.archive.file_names() {
            if let Some(first_component) = Path::new(name).components().next() {
                dirs.insert(first_component.as_os_str().to_string_lossy().to_string());
            }
        }

        dirs.into_iter().collect()
    }

    /// Get a `Vec` containing the files in the specified directory in the ZIP archive.
    pub fn files_in_directory<P: AsRef<Path>>(&mut self, dir: P) -> Vec<String> {
        let mut files = Vec::new();

        for file_name in self.archive.file_names() {
            if logical_is_path_inside(&dir, file_name) {
                files.push(file_name.to_string());
            }
        }

        files
    }

    /// Get the number of files in the ZIP archive.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.archive.len()
    }

    /// Returns `true` if the archive contains no files.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.archive.is_empty()
    }
}

/// Check if a given child path is logically inside a parent path.
#[inline]
fn logical_is_path_inside<P: AsRef<Path>, Q: AsRef<Path>>(parent: P, child: Q) -> bool {
    let parent = parent.as_ref().components().collect::<Vec<_>>();
    let child = child.as_ref().components().collect::<Vec<_>>();

    if parent.len() > child.len() {
        return false;
    }

    for (p, c) in parent.iter().zip(child.iter()) {
        if p != c {
            return false;
        }
    }

    true
}
