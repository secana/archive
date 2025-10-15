//! A unified interface for extracting common archive formats in-memory
//!
//! This crate provides a simple API for extracting various archive formats
//! including ZIP, TAR, and compressed TAR files, all in-memory without
//! touching the disk.

pub mod error;
pub mod extractor;
pub mod format;

pub use error::{ArchiveError, Result};
pub use extractor::{ArchiveExtractor, ExtractedFile};
pub use format::ArchiveFormat;
