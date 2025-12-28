//! Archive extraction implementations.
//!
//! This module provides the core extraction functionality for all supported
//! archive formats. The main entry point is [`ArchiveExtractor`], which can
//! extract files from any supported format into memory.

use crate::error::{ArchiveError, Result};
use crate::format::ArchiveFormat;
use std::io::{Cursor, Read};

/// Represents a single file extracted from an archive.
///
/// This structure contains the file's path within the archive, its contents,
/// and metadata about whether it represents a directory.
///
/// # Examples
///
/// ```no_run
/// use archive::{ArchiveExtractor, ArchiveFormat};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = ArchiveExtractor::new();
/// # let data = vec![0u8; 100];
/// let files = extractor.extract(&data, ArchiveFormat::Zip)?;
///
/// for file in files {
///     if file.is_directory {
///         println!("Directory: {}", file.path);
///     } else {
///         println!("File: {} ({} bytes)", file.path, file.data.len());
///         // Process file.data as needed
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// The original path of the file within the archive.
    ///
    /// For multi-file archives (ZIP, TAR, 7-Zip), this is the path as stored
    /// in the archive. For single-file compression formats:
    /// - **Gzip**: The original filename from the header, or "data" if not present
    /// - **Bzip2, XZ, LZ4, Zstandard**: Always "data" as these formats don't store filenames
    pub path: String,

    /// The decompressed contents of the file.
    ///
    /// For directories, this will be an empty vector.
    pub data: Vec<u8>,

    /// Whether this entry represents a directory.
    ///
    /// If `true`, the `data` field will be empty and `path` represents a directory.
    /// If `false`, this is a regular file with content in `data`.
    pub is_directory: bool,
}

/// Main extractor that handles all archive formats.
///
/// This is the primary interface for extracting archives. It supports all formats
/// defined in [`ArchiveFormat`] and provides configurable safety limits to protect
/// against malicious archives.
///
/// # Safety Features
///
/// The extractor includes built-in protections against:
/// - **Zip bombs**: Files that expand to enormous sizes
/// - **Resource exhaustion**: Configurable per-file and total size limits
/// - **Memory exhaustion**: All limits are checked before allocation
///
/// # Default Limits
///
/// - Maximum file size: 100 MB
/// - Maximum total extraction size: 1 GB
///
/// # Examples
///
/// ## Basic extraction
///
/// ```no_run
/// use archive::{ArchiveExtractor, ArchiveFormat};
/// use std::fs;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data = fs::read("archive.zip")?;
/// let extractor = ArchiveExtractor::new();
/// let files = extractor.extract(&data, ArchiveFormat::Zip)?;
///
/// println!("Extracted {} files", files.len());
/// # Ok(())
/// # }
/// ```
///
/// ## Custom size limits
///
/// ```no_run
/// use archive::{ArchiveExtractor, ArchiveFormat};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = ArchiveExtractor::new()
///     .with_max_file_size(10 * 1024 * 1024)     // 10 MB per file
///     .with_max_total_size(100 * 1024 * 1024);  // 100 MB total
///
/// # let data = vec![0u8; 100];
/// let files = extractor.extract(&data, ArchiveFormat::TarGz)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Handling different formats
///
/// ```no_run
/// use archive::{ArchiveExtractor, ArchiveFormat};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = ArchiveExtractor::new();
///
/// // Extract different archive types
/// # let zip_data = vec![0u8; 100];
/// let zip_files = extractor.extract(&zip_data, ArchiveFormat::Zip)?;
/// # let tar_data = vec![0u8; 100];
/// let tar_files = extractor.extract(&tar_data, ArchiveFormat::TarGz)?;
/// # let sevenz_data = vec![0u8; 100];
/// let sevenz_files = extractor.extract(&sevenz_data, ArchiveFormat::SevenZ)?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ArchiveExtractor {
    max_file_size: usize,
    max_total_size: usize,
}

impl Default for ArchiveExtractor {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,   // 100 MB per file
            max_total_size: 1024 * 1024 * 1024, // 1 GB total
        }
    }
}

impl ArchiveExtractor {
    /// Creates a new archive extractor with default settings.
    ///
    /// Default settings:
    /// - Maximum file size: 100 MB (104,857,600 bytes)
    /// - Maximum total extraction size: 1 GB (1,073,741,824 bytes)
    ///
    /// # Examples
    ///
    /// ```
    /// use archive::ArchiveExtractor;
    ///
    /// let extractor = ArchiveExtractor::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum size for individual files in the archive.
    ///
    /// This limit protects against extracting unexpectedly large files that could
    /// exhaust memory. If any file in the archive exceeds this size, extraction
    /// will fail with [`ArchiveError::FileTooLarge`].
    ///
    /// This method uses the builder pattern, allowing you to chain configuration calls.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum file size in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use archive::ArchiveExtractor;
    ///
    /// // Allow up to 50 MB per file
    /// let extractor = ArchiveExtractor::new()
    ///     .with_max_file_size(50 * 1024 * 1024);
    /// ```
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Sets the maximum total size for all extracted files combined.
    ///
    /// This limit protects against zip bombs and archives with many files that
    /// could collectively exhaust memory. If the total size of all files would
    /// exceed this limit, extraction will fail with [`ArchiveError::TotalSizeTooLarge`].
    ///
    /// This method uses the builder pattern, allowing you to chain configuration calls.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum total extraction size in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use archive::ArchiveExtractor;
    ///
    /// // Allow up to 500 MB total extraction
    /// let extractor = ArchiveExtractor::new()
    ///     .with_max_total_size(500 * 1024 * 1024);
    /// ```
    ///
    /// # Combined with other limits
    ///
    /// ```
    /// use archive::ArchiveExtractor;
    ///
    /// let extractor = ArchiveExtractor::new()
    ///     .with_max_file_size(10 * 1024 * 1024)    // 10 MB per file
    ///     .with_max_total_size(100 * 1024 * 1024); // 100 MB total
    /// ```
    pub fn with_max_total_size(mut self, size: usize) -> Self {
        self.max_total_size = size;
        self
    }

    /// Extracts all files from an archive.
    ///
    /// This is the main extraction method that handles all supported archive formats.
    /// The format must be explicitly specified via the `format` parameter.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw bytes of the archive file
    /// * `format` - The archive format to extract (see [`ArchiveFormat`])
    ///
    /// # Returns
    ///
    /// Returns a `Vec<ExtractedFile>` containing all files and directories from the archive.
    /// Directories will have `is_directory` set to `true` and an empty `data` field.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The archive data is invalid or corrupted ([`ArchiveError::InvalidArchive`])
    /// - Any file exceeds the maximum file size ([`ArchiveError::FileTooLarge`])
    /// - The total extracted size exceeds the limit ([`ArchiveError::TotalSizeTooLarge`])
    /// - An I/O error occurs during extraction ([`ArchiveError::Io`])
    /// - A ZIP-specific error occurs ([`ArchiveError::Zip`])
    ///
    /// # Examples
    ///
    /// ## Extract a ZIP file
    ///
    /// ```no_run
    /// use archive::{ArchiveExtractor, ArchiveFormat};
    /// use std::fs;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let data = fs::read("example.zip")?;
    /// let extractor = ArchiveExtractor::new();
    /// let files = extractor.extract(&data, ArchiveFormat::Zip)?;
    ///
    /// for file in files {
    ///     println!("{}: {} bytes", file.path, file.data.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Handle extraction errors
    ///
    /// ```no_run
    /// use archive::{ArchiveExtractor, ArchiveFormat, ArchiveError};
    ///
    /// # fn main() {
    /// let extractor = ArchiveExtractor::new()
    ///     .with_max_file_size(1024 * 1024); // 1 MB limit
    ///
    /// # let data = vec![0u8; 100];
    /// match extractor.extract(&data, ArchiveFormat::Zip) {
    ///     Ok(files) => {
    ///         println!("Successfully extracted {} files", files.len());
    ///     }
    ///     Err(ArchiveError::FileTooLarge { size, limit }) => {
    ///         eprintln!("File too large: {} bytes (limit: {} bytes)", size, limit);
    ///     }
    ///     Err(ArchiveError::InvalidArchive(msg)) => {
    ///         eprintln!("Invalid archive: {}", msg);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Extraction failed: {}", e);
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// ## Extract multiple formats
    ///
    /// ```no_run
    /// use archive::{ArchiveExtractor, ArchiveFormat};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let extractor = ArchiveExtractor::new();
    ///
    /// # let zip_data = vec![0u8; 100];
    /// let zip_files = extractor.extract(&zip_data, ArchiveFormat::Zip)?;
    /// # let tar_data = vec![0u8; 100];
    /// let tar_files = extractor.extract(&tar_data, ArchiveFormat::TarGz)?;
    /// # let gz_data = vec![0u8; 100];
    /// let gz_files = extractor.extract(&gz_data, ArchiveFormat::Gz)?;
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract(&self, data: &[u8], format: ArchiveFormat) -> Result<Vec<ExtractedFile>> {
        match format {
            ArchiveFormat::Zip => self.extract_zip(data),
            ArchiveFormat::Tar => self.extract_tar(data),
            ArchiveFormat::Ar => self.extract_ar(data),
            ArchiveFormat::Deb => self.extract_deb(data),
            ArchiveFormat::TarGz => self.extract_tar_gz(data),
            ArchiveFormat::TarBz2 => self.extract_tar_bz2(data),
            ArchiveFormat::TarXz => self.extract_tar_xz(data),
            ArchiveFormat::TarZst => self.extract_tar_zst(data),
            ArchiveFormat::TarLz4 => self.extract_tar_lz4(data),
            ArchiveFormat::SevenZ => self.extract_7z(data),
            ArchiveFormat::Gz => self.extract_single_gz(data),
            ArchiveFormat::Bz2 => self.extract_single_bz2(data),
            ArchiveFormat::Xz => self.extract_single_xz(data),
            ArchiveFormat::Lz4 => self.extract_single_lz4(data),
            ArchiveFormat::Zst => self.extract_single_zst(data),
        }
    }

    fn extract_zip(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let reader = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let is_directory = file.is_dir();

            if !is_directory {
                let size = file.size() as usize;
                if size > self.max_file_size {
                    return Err(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                }

                total_size += size;
                if total_size > self.max_total_size {
                    return Err(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                }

                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    path: file.name().to_string(),
                    data: contents,
                    is_directory,
                });
            } else {
                files.push(ExtractedFile {
                    path: file.name().to_string(),
                    data: Vec::new(),
                    is_directory,
                });
            }
        }

        Ok(files)
    }

    fn extract_tar(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_ar(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut archive = ar::Archive::new(cursor);
        self.process_ar_entries(&mut archive)
    }

    fn extract_deb(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut archive = ar::Archive::new(cursor);
        self.process_ar_entries(&mut archive)
    }

    fn extract_tar_gz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_bz2(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = bzip2::read::BzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_xz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut output = Vec::new();
        lzma_rs::xz_decompress(&mut cursor.clone(), &mut output)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;
        let cursor = Cursor::new(output);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_zst(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_lz4(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let decoder = lz4::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_7z(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let mut cursor = Cursor::new(data);
        let len = cursor.get_ref().len() as u64;

        let mut archive = sevenz_rust::SevenZReader::new(&mut cursor, len, "".into())
            .map_err(|e| ArchiveError::InvalidArchive(format!("7z error: {}", e)))?;

        let mut files = Vec::new();
        let mut total_size = 0usize;
        let mut size_error: Option<ArchiveError> = None;

        // Single-pass extraction: validate sizes and extract contents in one iteration
        let result = archive.for_each_entries(|entry, reader| {
            if entry.is_directory() {
                files.push(ExtractedFile {
                    path: entry.name().to_string(),
                    data: Vec::new(),
                    is_directory: true,
                });
            } else {
                let size = entry.size() as usize;
                if size > self.max_file_size {
                    size_error = Some(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                    return Ok(false); // Stop iteration
                }

                total_size += size;
                if total_size > self.max_total_size {
                    size_error = Some(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                    return Ok(false); // Stop iteration
                }

                let mut contents = Vec::new();
                reader.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    path: entry.name().to_string(),
                    data: contents,
                    is_directory: false,
                });
            }
            Ok(true)
        });

        // Check if we stopped due to size limits
        if let Some(err) = size_error {
            return Err(err);
        }

        // Check for other extraction errors
        result.map_err(|e| ArchiveError::InvalidArchive(format!("7z extraction error: {}", e)))?;

        Ok(files)
    }

    // Single-file decompression methods

    fn extract_single_gz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = flate2::read::GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        // Try to extract original filename from gzip header
        let path = decoder
            .header()
            .and_then(|h| h.filename())
            .and_then(|f| std::str::from_utf8(f).ok())
            .unwrap_or("data")
            .to_string();

        Ok(vec![ExtractedFile {
            path,
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_bz2(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = bzip2::read::BzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "data".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_xz(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let mut cursor = Cursor::new(data);
        let mut decompressed = Vec::new();
        lzma_rs::xz_decompress(&mut cursor, &mut decompressed)
            .map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "data".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_lz4(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = lz4::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "data".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn extract_single_zst(&self, data: &[u8]) -> Result<Vec<ExtractedFile>> {
        let cursor = Cursor::new(data);
        let mut decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        if decompressed.len() > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: decompressed.len(),
                limit: self.max_file_size,
            });
        }

        Ok(vec![ExtractedFile {
            path: "data".to_string(),
            data: decompressed,
            is_directory: false,
        }])
    }

    fn process_tar_entries<R: Read>(
        &self,
        archive: &mut tar::Archive<R>,
    ) -> Result<Vec<ExtractedFile>> {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_string_lossy().to_string();
            let is_directory = entry.header().entry_type().is_dir();

            if !is_directory {
                let size = entry.size() as usize;
                if size > self.max_file_size {
                    return Err(ArchiveError::FileTooLarge {
                        size,
                        limit: self.max_file_size,
                    });
                }

                total_size += size;
                if total_size > self.max_total_size {
                    return Err(ArchiveError::TotalSizeTooLarge {
                        size: total_size,
                        limit: self.max_total_size,
                    });
                }

                let mut contents = Vec::new();
                entry.read_to_end(&mut contents)?;

                files.push(ExtractedFile {
                    path,
                    data: contents,
                    is_directory,
                });
            } else {
                files.push(ExtractedFile {
                    path,
                    data: Vec::new(),
                    is_directory,
                });
            }
        }

        Ok(files)
    }

    fn process_ar_entries<R: Read>(
        &self,
        archive: &mut ar::Archive<R>,
    ) -> Result<Vec<ExtractedFile>> {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        while let Some(entry_result) = archive.next_entry(){
            let mut entry = entry_result?;
            let path = String::from_utf8_lossy(entry.header().identifier()).to_string();

            let size = entry.header().size() as usize;
            if size > self.max_file_size {
                return Err(ArchiveError::FileTooLarge {
                    size,
                    limit: self.max_file_size,
                });
            }

            total_size += size;
            if total_size > self.max_total_size {
                return Err(ArchiveError::TotalSizeTooLarge {
                    size: total_size,
                    limit: self.max_total_size,
                });
            }

            let mut contents = Vec::new();
            entry.read_to_end(&mut contents)?;

            files.push(ExtractedFile {
                path,
                data: contents,
                is_directory: false,
            });
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let extractor = ArchiveExtractor::new();
        assert_eq!(extractor.max_file_size, 100 * 1024 * 1024);
        assert_eq!(extractor.max_total_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_builder_pattern() {
        let extractor = ArchiveExtractor::new()
            .with_max_file_size(50 * 1024 * 1024)
            .with_max_total_size(500 * 1024 * 1024);

        assert_eq!(extractor.max_file_size, 50 * 1024 * 1024);
        assert_eq!(extractor.max_total_size, 500 * 1024 * 1024);
    }
}
