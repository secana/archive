//! Error types for archive operations.
//!
//! This module provides error types used throughout the crate for handling
//! various failure scenarios during archive extraction.

use std::io;
use thiserror::Error;

/// Result type alias for archive operations.
///
/// This is a convenience type that uses [`ArchiveError`] as the error type.
///
/// # Examples
///
/// ```
/// use archive::{Result, ArchiveError};
///
/// fn extract_something() -> Result<Vec<u8>> {
///     // Your extraction logic
///     Ok(vec![])
/// }
/// ```
pub type Result<T> = std::result::Result<T, ArchiveError>;

/// Errors that can occur during archive extraction.
///
/// This enum represents all possible errors that can occur when working with
/// archives, including I/O errors, format-specific errors, and safety limit violations.
///
/// # Examples
///
/// ```
/// use archive::{ArchiveExtractor, ArchiveFormat, ArchiveError};
///
/// # fn main() {
/// let extractor = ArchiveExtractor::new()
///     .with_max_file_size(1024); // Very small limit for demo
///
/// # let data = vec![0u8; 100];
/// match extractor.extract(&data, ArchiveFormat::Zip) {
///     Ok(files) => println!("Success: {} files", files.len()),
///     Err(ArchiveError::FileTooLarge { size, limit }) => {
///         eprintln!("File of {} bytes exceeds limit of {}", size, limit);
///     }
///     Err(ArchiveError::InvalidArchive(msg)) => {
///         eprintln!("Invalid archive: {}", msg);
///     }
///     Err(e) => eprintln!("Other error: {}", e),
/// }
/// # }
/// ```
#[derive(Error, Debug)]
pub enum ArchiveError {
    /// An I/O error occurred while reading or processing archive data.
    ///
    /// This can occur during file reading, decompression, or other I/O operations.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// A ZIP-specific error occurred.
    ///
    /// This wraps errors from the underlying ZIP library, such as
    /// corruption, unsupported features, or encryption issues.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// The archive format could not be determined.
    ///
    /// This error is currently unused but reserved for future auto-detection features.
    #[error("Unknown archive format")]
    UnknownFormat,

    /// A single file in the archive exceeds the configured size limit.
    ///
    /// This is a safety feature to prevent memory exhaustion from malicious
    /// or accidentally large files. The limit can be configured using
    /// [`ArchiveExtractor::with_max_file_size`](crate::ArchiveExtractor::with_max_file_size).
    ///
    /// # Fields
    ///
    /// - `size`: The actual size of the file in bytes
    /// - `limit`: The configured maximum file size in bytes
    #[error("File too large: {size} bytes exceeds limit of {limit} bytes")]
    FileTooLarge {
        /// The actual size of the file that exceeded the limit
        size: usize,
        /// The configured maximum file size
        limit: usize,
    },

    /// The total size of all extracted files exceeds the configured limit.
    ///
    /// This is a safety feature to prevent memory exhaustion from zip bombs
    /// or archives with many files. The limit can be configured using
    /// [`ArchiveExtractor::with_max_total_size`](crate::ArchiveExtractor::with_max_total_size).
    ///
    /// # Fields
    ///
    /// - `size`: The total size that would be extracted in bytes
    /// - `limit`: The configured maximum total size in bytes
    #[error("Total extraction size {size} bytes exceeds limit of {limit} bytes")]
    TotalSizeTooLarge {
        /// The total size that exceeded the limit
        size: usize,
        /// The configured maximum total extraction size
        limit: usize,
    },

    /// The archive is invalid or corrupted.
    ///
    /// This error occurs when the archive data doesn't conform to the expected
    /// format specification, is corrupted, or cannot be properly parsed.
    ///
    /// The string contains a detailed error message about what went wrong.
    #[error("Invalid archive: {0}")]
    InvalidArchive(String),

    /// The archive format or feature is not supported.
    ///
    /// This error occurs when attempting to extract an archive with features
    /// that are not yet implemented, such as certain encryption schemes or
    /// compression methods.
    ///
    /// The string contains details about what is unsupported.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
