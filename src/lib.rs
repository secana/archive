//! A unified interface for extracting common archive formats in-memory.
//!
//! This crate provides a simple, safe API for extracting various archive formats
//! including ZIP, TAR (with multiple compression options), 7-Zip, and single-file
//! compression formats. All extraction happens in-memory without touching the disk.
//!
//! # Features
//!
//! - **Unified API**: Single interface for all archive formats
//! - **In-memory extraction**: No disk I/O required
//! - **Safety limits**: Protection against zip bombs and resource exhaustion
//! - **Pure Rust**: Minimal C dependencies (only bzip2)
//! - **Cross-platform**: Works on Linux, macOS, Windows (x86_64, ARM64)
//!
//! # Supported Formats
//!
//! - **ZIP** (`.zip`)
//! - **TAR** (`.tar`, `.tar.gz`, `.tar.bz2`, `.tar.xz`, `.tar.zst`, `.tar.lz4`)
//! - **7-Zip** (`.7z`)
//! - **Single-file compression** (`.gz`, `.bz2`, `.xz`, `.lz4`, `.zst`)
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use archive::{ArchiveExtractor, ArchiveFormat};
//! use std::fs;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Read archive file
//! let data = fs::read("example.zip")?;
//!
//! // Create extractor with default settings
//! let extractor = ArchiveExtractor::new();
//!
//! // Extract all files
//! let files = extractor.extract(&data, ArchiveFormat::Zip)?;
//!
//! // Process extracted files
//! for file in files {
//!     if !file.is_directory {
//!         println!("File: {} ({} bytes)", file.path, file.data.len());
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Size Limits
//!
//! Protect against zip bombs and resource exhaustion:
//!
//! ```no_run
//! use archive::{ArchiveExtractor, ArchiveFormat};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let data = vec![0u8; 100];
//! let extractor = ArchiveExtractor::new()
//!     .with_max_file_size(50 * 1024 * 1024)      // 50 MB per file
//!     .with_max_total_size(500 * 1024 * 1024);   // 500 MB total
//!
//! let files = extractor.extract(&data, ArchiveFormat::Zip)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Multiple Archive Formats
//!
//! ```no_run
//! use archive::{ArchiveExtractor, ArchiveFormat};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let extractor = ArchiveExtractor::new();
//!
//! // Extract ZIP archive
//! # let zip_data = vec![0u8; 100];
//! let zip_files = extractor.extract(&zip_data, ArchiveFormat::Zip)?;
//!
//! // Extract TAR.GZ archive
//! # let targz_data = vec![0u8; 100];
//! let tar_files = extractor.extract(&targz_data, ArchiveFormat::TarGz)?;
//!
//! // Extract 7-Zip archive
//! # let sevenz_data = vec![0u8; 100];
//! let seven_files = extractor.extract(&sevenz_data, ArchiveFormat::SevenZ)?;
//!
//! // Decompress single gzip file
//! # let gz_data = vec![0u8; 100];
//! let gz_files = extractor.extract(&gz_data, ArchiveFormat::Gz)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Safety
//!
//! This crate includes built-in protections against:
//! - **Zip bombs**: Files that expand to enormous sizes
//! - **Resource exhaustion**: Configurable size limits
//! - **Path traversal**: Safe handling of archive paths
//!
//! Default limits:
//! - Maximum file size: 100 MB
//! - Maximum total extraction size: 1 GB
//!
//! # Error Handling
//!
//! ```no_run
//! use archive::{ArchiveExtractor, ArchiveFormat, ArchiveError};
//!
//! # fn main() {
//! let extractor = ArchiveExtractor::new()
//!     .with_max_file_size(1024 * 1024); // 1 MB limit
//!
//! # let data = vec![0u8; 100];
//! match extractor.extract(&data, ArchiveFormat::Zip) {
//!     Ok(files) => println!("Extracted {} files", files.len()),
//!     Err(ArchiveError::FileTooLarge { size, limit }) => {
//!         eprintln!("File too large: {} bytes (limit: {})", size, limit);
//!     }
//!     Err(e) => eprintln!("Extraction failed: {}", e),
//! }
//! # }
//! ```

pub mod error;
pub mod extractor;
pub mod format;

pub use error::{ArchiveError, Result};
pub use extractor::{ArchiveExtractor, ExtractedFile};
pub use format::ArchiveFormat;
