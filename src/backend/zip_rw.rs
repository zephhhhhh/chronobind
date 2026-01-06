use filesystem::File;
use std::fs as filesystem;
use std::path::Path;

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
    pub fn finish(mut self) -> AnyResult<()> {
        if let Some(zip) = self.zip.take() {
            zip.finish()?;
        }
        Ok(())
    }
}
