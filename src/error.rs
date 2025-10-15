//! Error types for archive operations

use std::io;
use thiserror::Error;

/// Result type alias for archive operations
pub type Result<T> = std::result::Result<T, ArchiveError>;

/// Errors that can occur during archive extraction
#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Unknown archive format")]
    UnknownFormat,

    #[error("File too large: {size} bytes exceeds limit of {limit} bytes")]
    FileTooLarge { size: usize, limit: usize },

    #[error("Total extraction size {size} bytes exceeds limit of {limit} bytes")]
    TotalSizeTooLarge { size: usize, limit: usize },

    #[error("Invalid archive: {0}")]
    InvalidArchive(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
