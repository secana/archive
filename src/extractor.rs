//! Archive extraction implementations.
//!
//! This module provides the core extraction functionality for all supported
//! archive formats. The main entry point is [`ArchiveExtractor`], which can
//! extract files from any supported format into memory.

use crate::error::{ArchiveError, Result};
use crate::format::ArchiveFormat;
use crate::path_safety::{validate_links, validate_path};
use std::io::{Cursor, Read, Write};

/// Unix mode bits identifying a symbolic link (`S_IFLNK`), as stored in the
/// upper 16 bits of a ZIP entry's `unix_mode()`.
const S_IFLNK: u32 = 0o120000;
const S_IFMT: u32 = 0o170000;

/// Extra slack (bytes) added to `max_total_size` when bounding the raw
/// decompressed `.tar.xz` byte stream. `lzma-rs` only exposes a one-shot,
/// fully-buffered decompress call, so this bounds the *tar container*
/// (data + 512-byte block headers/padding) before it's parsed into
/// per-entry sizes; the allowance covers header overhead on archives with
/// many small files. Individual entries are still checked precisely
/// afterwards in [`ArchiveExtractor::process_tar_entries`].
///
/// Because the container is buffered before its entries are copied out,
/// peak memory for `.tar.xz` extraction can reach roughly twice
/// `max_total_size`.
const TAR_XZ_HEADER_OVERHEAD_ALLOWANCE: usize = 64 * 1024 * 1024;

/// A `Write` sink that stops accepting bytes once more than `limit` have
/// been written, so a one-shot streaming decompressor (like `lzma-rs`'s
/// `xz_decompress`) can be interrupted before a decompression bomb finishes
/// allocating unbounded memory, instead of only being caught by a size
/// check *after* decompression has already completed.
struct BoundedWriter<'a> {
    buf: &'a mut Vec<u8>,
    limit: usize,
    total_seen: usize,
}

impl<'a> BoundedWriter<'a> {
    fn new(buf: &'a mut Vec<u8>, limit: usize) -> Self {
        Self {
            buf,
            limit,
            total_seen: 0,
        }
    }
}

impl Write for BoundedWriter<'_> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.total_seen += data.len();
        if self.total_seen > self.limit {
            return Err(std::io::Error::other(
                "decompressed size exceeds configured limit",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Reserves exactly `size` bytes of capacity in `buf` up front, so an entry
/// that's within the configured limit but too large for the allocator to
/// actually provide right now surfaces as [`ArchiveError::AllocationFailed`]
/// instead of aborting the process.
fn reserve_exact(buf: &mut Vec<u8>, size: usize) -> Result<()> {
    buf.try_reserve_exact(size)
        .map_err(|source| ArchiveError::AllocationFailed { size, source })
}

/// Reads all of `reader` into `buf`, aborting with
/// [`ArchiveError::FileTooLarge`] as soon as more than `limit` bytes have
/// been produced, instead of letting a decompression bomb fully materialize
/// in memory before the size is checked.
fn bounded_read_to_end(
    reader: &mut (impl Read + ?Sized),
    buf: &mut Vec<u8>,
    limit: usize,
) -> Result<()> {
    let mut writer = BoundedWriter::new(buf, limit);
    let result = std::io::copy(reader, &mut writer);
    let total_seen = writer.total_seen;
    result.map(|_| ()).map_err(|e| {
        if total_seen > limit {
            ArchiveError::FileTooLarge {
                size: total_seen,
                limit,
            }
        } else {
            ArchiveError::Io(e)
        }
    })
}

/// A single entry extracted from an archive.
///
/// Each variant carries only the fields that make sense for that kind of
/// entry, so states like "directory with file contents" or "symlink with no
/// target" can't be represented.
///
/// # Examples
///
/// ```no_run
/// use archive::{ArchiveEntry, ArchiveExtractor, ArchiveFormat};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = ArchiveExtractor::new();
/// # let data = vec![0u8; 100];
/// let entries = extractor.extract(&data, ArchiveFormat::Zip)?;
///
/// for entry in &entries {
///     match entry {
///         ArchiveEntry::File { path, data } => {
///             println!("File: {} ({} bytes)", path, data.len());
///         }
///         ArchiveEntry::Directory { path } => {
///             println!("Directory: {}", path);
///         }
///         ArchiveEntry::Symlink { path, target } => {
///             println!("Symlink: {} -> {}", path, target);
///         }
///         ArchiveEntry::HardLink { path, target } => {
///             println!("Hard link: {} => {}", path, target);
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub enum ArchiveEntry {
    /// A regular file with decompressed contents.
    File {
        /// The path of the file within the archive.
        ///
        /// For multi-file archives (ZIP, TAR, 7-Zip), this is the path as
        /// stored in the archive. For single-file compression formats:
        /// - **Gzip**: The original filename from the header, or "data" if not present
        /// - **Bzip2, XZ, LZ4, Zstandard**: Always "data" as these formats don't store filenames
        path: String,
        /// The decompressed contents of the file.
        data: Vec<u8>,
    },

    /// A directory entry.
    Directory {
        /// The path of the directory within the archive.
        path: String,
    },

    /// A symbolic link entry.
    ///
    /// The target is relative to the directory containing the symlink. It
    /// has been validated to be relative and to resolve inside the
    /// extraction root, and no other entry of the archive is placed beneath
    /// a symlink. Extraction fails with [`ArchiveError::UnsafePath`] instead
    /// of producing an unsafe symlink.
    ///
    /// These checks only cover the archive itself. Callers that materialize
    /// entries onto a filesystem should still skip symlinks or re-validate
    /// them against their own extraction root, because existing files in
    /// that directory are not taken into account.
    Symlink {
        /// The path of the symlink within the archive.
        path: String,
        /// The link target, as recorded in the archive.
        target: String,
    },

    /// A hard link entry (TAR only).
    ///
    /// Unlike a symlink target, the target is relative to the extraction
    /// root and names another entry of the archive. It has been validated to
    /// contain no `..` component and to not pass through a symlink.
    HardLink {
        /// The path of the hard link within the archive.
        path: String,
        /// The path of the linked entry within the archive.
        target: String,
    },
}

impl ArchiveEntry {
    /// Returns the path of this entry within the archive.
    pub fn path(&self) -> &str {
        match self {
            ArchiveEntry::File { path, .. } => path,
            ArchiveEntry::Directory { path } => path,
            ArchiveEntry::Symlink { path, .. } => path,
            ArchiveEntry::HardLink { path, .. } => path,
        }
    }

    /// Returns `true` if this entry is a regular file.
    pub fn is_file(&self) -> bool {
        matches!(self, ArchiveEntry::File { .. })
    }

    /// Returns `true` if this entry is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self, ArchiveEntry::Directory { .. })
    }

    /// Returns `true` if this entry is a symlink.
    pub fn is_symlink(&self) -> bool {
        matches!(self, ArchiveEntry::Symlink { .. })
    }

    /// Returns `true` if this entry is a hard link.
    pub fn is_hard_link(&self) -> bool {
        matches!(self, ArchiveEntry::HardLink { .. })
    }

    /// Returns the file contents, or `None` if this entry isn't a [`ArchiveEntry::File`].
    pub fn data(&self) -> Option<&[u8]> {
        match self {
            ArchiveEntry::File { data, .. } => Some(data),
            _ => None,
        }
    }
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
/// - **Memory exhaustion**: Limits are enforced against the bytes actually
///   decompressed, not the sizes declared in the archive
/// - **Path traversal**: Entry paths and link targets must stay inside the
///   extraction root
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
    /// Returns a `Vec<ArchiveEntry>` containing all files, directories, and links
    /// from the archive.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The archive data is invalid or corrupted ([`ArchiveError::InvalidArchive`])
    /// - Any file exceeds the maximum file size ([`ArchiveError::FileTooLarge`])
    /// - The total extracted size exceeds the limit ([`ArchiveError::TotalSizeTooLarge`])
    /// - An entry path or link target would escape the extraction root
    ///   ([`ArchiveError::UnsafePath`])
    /// - A buffer for an entry cannot be allocated ([`ArchiveError::AllocationFailed`])
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
    /// for entry in &files {
    ///     println!("{}: {} bytes", entry.path(), entry.data().map_or(0, <[u8]>::len));
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
    pub fn extract(&self, data: &[u8], format: ArchiveFormat) -> Result<Vec<ArchiveEntry>> {
        let entries = match format {
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
        }?;

        validate_links(&entries)?;
        Ok(entries)
    }

    /// Reads the contents of one archive entry.
    ///
    /// The declared size is untrusted archive metadata: it is only used to
    /// reject an obviously too large claim up front and to size the buffer.
    /// The actual bytes produced are bounded by both `max_file_size` and the
    /// space left in `max_total_size`, regardless of what is declared.
    fn read_entry(
        &self,
        reader: &mut (impl Read + ?Sized),
        declared_size: u64,
        total_size: &mut usize,
    ) -> Result<Vec<u8>> {
        let declared_size = usize::try_from(declared_size).unwrap_or(usize::MAX);
        if declared_size > self.max_file_size {
            return Err(ArchiveError::FileTooLarge {
                size: declared_size,
                limit: self.max_file_size,
            });
        }

        let remaining_total = self.max_total_size.saturating_sub(*total_size);
        if declared_size > remaining_total {
            return Err(ArchiveError::TotalSizeTooLarge {
                size: total_size.saturating_add(declared_size),
                limit: self.max_total_size,
            });
        }

        let limit = self.max_file_size.min(remaining_total);
        let mut contents = Vec::new();
        reserve_exact(&mut contents, declared_size)?;
        bounded_read_to_end(reader, &mut contents, limit).map_err(|e| match e {
            ArchiveError::FileTooLarge { size, .. } if size <= self.max_file_size => {
                ArchiveError::TotalSizeTooLarge {
                    size: total_size.saturating_add(size),
                    limit: self.max_total_size,
                }
            }
            ArchiveError::FileTooLarge { size, .. } => ArchiveError::FileTooLarge {
                size,
                limit: self.max_file_size,
            },
            other => other,
        })?;

        // An entry that declares more than it contains must not keep the
        // unused capacity, or many small entries could hold far more memory
        // than `max_total_size` allows.
        contents.shrink_to_fit();

        *total_size += contents.len();
        Ok(contents)
    }

    fn extract_zip(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let reader = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let is_directory = file.is_dir();
            let path = file.name()?.into_owned();
            validate_path(&path)?;

            let is_symlink = file
                .unix_mode()
                .is_some_and(|mode| mode & S_IFMT == S_IFLNK);

            if !is_directory {
                let size = file.size();
                let contents = self.read_entry(&mut file, size, &mut total_size)?;

                if is_symlink {
                    let target = String::from_utf8_lossy(&contents).into_owned();
                    files.push(ArchiveEntry::Symlink { path, target });
                } else {
                    files.push(ArchiveEntry::File {
                        path,
                        data: contents,
                    });
                }
            } else {
                files.push(ArchiveEntry::Directory { path });
            }
        }

        Ok(files)
    }

    fn extract_tar(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_ar(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut archive = ar::Archive::new(cursor);
        self.process_ar_entries(&mut archive)
    }

    fn extract_deb(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut archive = ar::Archive::new(cursor);
        self.process_ar_entries(&mut archive)
    }

    fn extract_tar_gz(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let decoder = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_bz2(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let decoder = bzip2::read::BzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_xz(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let mut cursor = Cursor::new(data);
        let limit = self
            .max_total_size
            .saturating_add(TAR_XZ_HEADER_OVERHEAD_ALLOWANCE);

        let mut output = Vec::new();
        let (result, total_seen) = {
            let mut writer = BoundedWriter::new(&mut output, limit);
            let result = lzma_rs::xz_decompress(&mut cursor, &mut writer);
            (result, writer.total_seen)
        };
        result.map_err(|e| {
            if total_seen > limit {
                ArchiveError::TotalSizeTooLarge {
                    size: total_seen,
                    limit,
                }
            } else {
                ArchiveError::InvalidArchive(e.to_string())
            }
        })?;

        let cursor = Cursor::new(output);
        let mut archive = tar::Archive::new(cursor);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_zst(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_tar_lz4(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let decoder = lz4::Decoder::new(cursor)?;
        let mut archive = tar::Archive::new(decoder);
        self.process_tar_entries(&mut archive)
    }

    fn extract_7z(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let mut cursor = Cursor::new(data);
        let len = cursor.get_ref().len() as u64;

        let mut archive = sevenz_rust::SevenZReader::new(&mut cursor, len, "".into())
            .map_err(|e| ArchiveError::InvalidArchive(format!("7z error: {}", e)))?;

        let mut files = Vec::new();
        let mut total_size = 0usize;
        let mut early_error: Option<ArchiveError> = None;

        // Single-pass extraction: validate sizes and extract contents in one iteration
        let result = archive.for_each_entries(|entry, reader| {
            let path = entry.name().to_string();
            if let Err(e) = validate_path(&path) {
                early_error = Some(e);
                return Ok(false); // Stop iteration
            }

            if entry.is_directory() {
                files.push(ArchiveEntry::Directory { path });
            } else {
                let contents = match self.read_entry(reader, entry.size(), &mut total_size) {
                    Ok(contents) => contents,
                    Err(e) => {
                        early_error = Some(e);
                        return Ok(false); // Stop iteration
                    }
                };

                files.push(ArchiveEntry::File {
                    path,
                    data: contents,
                });
            }
            Ok(true)
        });

        // Check if we stopped due to a size or path-safety violation
        if let Some(err) = early_error {
            return Err(err);
        }

        // Check for other extraction errors
        result.map_err(|e| ArchiveError::InvalidArchive(format!("7z extraction error: {}", e)))?;

        Ok(files)
    }

    // Single-file decompression methods

    fn extract_single_gz(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut decoder = flate2::read::GzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        bounded_read_to_end(&mut decoder, &mut decompressed, self.max_file_size)?;

        // Try to extract original filename from gzip header. Fall back to
        // "data" if it's missing, not valid UTF-8, or unsafe (a gzip header
        // filename is attacker-controlled and could contain e.g. "../..").
        let path = decoder
            .header()
            .and_then(|h| h.filename())
            .and_then(|f| std::str::from_utf8(f).ok())
            .filter(|f| validate_path(f).is_ok())
            .unwrap_or("data")
            .to_string();

        Ok(vec![ArchiveEntry::File {
            path,
            data: decompressed,
        }])
    }

    fn extract_single_bz2(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut decoder = bzip2::read::BzDecoder::new(cursor);
        let mut decompressed = Vec::new();
        bounded_read_to_end(&mut decoder, &mut decompressed, self.max_file_size)?;

        Ok(vec![ArchiveEntry::File {
            path: "data".to_string(),
            data: decompressed,
        }])
    }

    fn extract_single_xz(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let mut cursor = Cursor::new(data);
        let mut decompressed = Vec::new();
        let (result, total_seen) = {
            let mut writer = BoundedWriter::new(&mut decompressed, self.max_file_size);
            let result = lzma_rs::xz_decompress(&mut cursor, &mut writer);
            (result, writer.total_seen)
        };
        result.map_err(|e| {
            if total_seen > self.max_file_size {
                ArchiveError::FileTooLarge {
                    size: total_seen,
                    limit: self.max_file_size,
                }
            } else {
                ArchiveError::InvalidArchive(e.to_string())
            }
        })?;

        Ok(vec![ArchiveEntry::File {
            path: "data".to_string(),
            data: decompressed,
        }])
    }

    fn extract_single_lz4(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut decoder = lz4::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        bounded_read_to_end(&mut decoder, &mut decompressed, self.max_file_size)?;

        Ok(vec![ArchiveEntry::File {
            path: "data".to_string(),
            data: decompressed,
        }])
    }

    fn extract_single_zst(&self, data: &[u8]) -> Result<Vec<ArchiveEntry>> {
        let cursor = Cursor::new(data);
        let mut decoder = zstd::stream::read::Decoder::new(cursor)?;
        let mut decompressed = Vec::new();
        bounded_read_to_end(&mut decoder, &mut decompressed, self.max_file_size)?;

        Ok(vec![ArchiveEntry::File {
            path: "data".to_string(),
            data: decompressed,
        }])
    }

    fn process_tar_entries<R: Read>(
        &self,
        archive: &mut tar::Archive<R>,
    ) -> Result<Vec<ArchiveEntry>> {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_string_lossy().to_string();
            validate_path(&path)?;

            let entry_type = entry.header().entry_type();

            if entry_type.is_symlink() || entry_type.is_hard_link() {
                let target = entry
                    .link_name()?
                    .map(|t| t.to_string_lossy().to_string())
                    .ok_or_else(|| {
                        ArchiveError::InvalidArchive(format!("link without target: {path}"))
                    })?;

                if entry_type.is_hard_link() {
                    validate_path(&target)?;
                    files.push(ArchiveEntry::HardLink { path, target });
                } else {
                    files.push(ArchiveEntry::Symlink { path, target });
                }
            } else if entry_type.is_dir() {
                files.push(ArchiveEntry::Directory { path });
            } else {
                let size = entry.size();
                let contents = self.read_entry(&mut entry, size, &mut total_size)?;
                files.push(ArchiveEntry::File {
                    path,
                    data: contents,
                });
            }
        }

        Ok(files)
    }

    fn process_ar_entries<R: Read>(
        &self,
        archive: &mut ar::Archive<R>,
    ) -> Result<Vec<ArchiveEntry>> {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        while let Some(entry_result) = archive.next_entry() {
            let mut entry = entry_result?;
            let path = String::from_utf8_lossy(entry.header().identifier()).to_string();
            validate_path(&path)?;

            let size = entry.header().size();
            let contents = self.read_entry(&mut entry, size, &mut total_size)?;

            files.push(ArchiveEntry::File {
                path,
                data: contents,
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
